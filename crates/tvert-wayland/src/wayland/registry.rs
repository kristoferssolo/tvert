use crate::{
    OutputId,
    global::Global,
    state::State,
    wayland::{OutputState, output::OutputData},
};
use std::{collections::HashMap, ops::RangeInclusive};
use tracing::{trace, warn};
use wayland_client::{
    Dispatch, Proxy, QueueHandle,
    globals::GlobalListContents,
    protocol::{
        wl_output::WlOutput,
        wl_registry::{Event, WlRegistry},
    },
};

impl State {
    pub fn bind_advertised_globals(&mut self, registry: &WlRegistry, qh: &QueueHandle<Self>) {
        for global in self.globals.clone() {
            self.bind_global(registry, qh, &global);
        }
        self.request_xdg_outputs(qh);
    }

    fn bind_global(&mut self, registry: &WlRegistry, qh: &QueueHandle<Self>, global: &Global) {
        if global.interface == WlOutput::interface().name {
            self.bind_output(registry, qh, global);
            return;
        }

        let mut binder = Binder {
            registry,
            qh,
            global,
            bound_globals: &mut self.bound_globals,
        };
        binder.bind(&mut self.shm, 1..=2);
        binder.bind(&mut self.screencopy_manager, 1..=3);
        binder.bind(&mut self.compositor, 4..=4);
        binder.bind(&mut self.layer_shell, 1..=4);
        binder.bind(&mut self.viewporter, 1..=1);
        binder.bind(&mut self.xdg_output_manager, 1..=3);
        binder.bind(&mut self.seat, 5..=5);
        binder.bind(&mut self.cursor_shape_manager, 1..=1);
    }

    fn bind_output(&mut self, registry: &WlRegistry, qh: &QueueHandle<Self>, global: &Global) {
        let id = OutputId(global.name);

        if self.output_states.contains_key(&id) {
            return;
        }

        if global.version < 4 {
            warn!(
                name = global.name,
                advertising = global.version,
                required = 4,
                "skipping output without wl_output name support"
            );
            return;
        }

        let proxy = registry.bind(global.name, 4, qh, OutputData { id });
        self.output_states.insert(id, OutputState::new(proxy));
    }

    fn remove_global(&mut self, name: u32) {
        let interface = self
            .globals
            .iter()
            .find(|global| global.name == name)
            .map(|global| global.interface.clone());

        let Some(interface) = interface else { return };

        self.globals.retain(|global| global.name != name);

        if interface == WlOutput::interface().name {
            self.remove_output(OutputId(name));
            return;
        }

        self.remove_singleton(name);
    }

    fn remove_output(&mut self, id: OutputId) {
        self.screencopy.cancel_for_output(id);

        drop(self.overlays.remove(&id));

        self.outputs.remove(&id);

        let Some(output) = self.output_states.remove(&id) else {
            return;
        };

        if let Some(xdg_output) = output.xdg_output {
            xdg_output.destroy();
        }

        if output.proxy.version() >= 3 {
            output.proxy.release();
        }
    }

    fn remove_singleton(&mut self, name: u32) {
        if let Some(shm) = take_removed(&mut self.shm, &mut self.bound_globals, name) {
            self.shm_formats.clear();

            if shm.version() >= 2 {
                shm.release();
            }
        }

        macro_rules! destroy_removed {
            ($field:ident) => {
                if let Some(manager) = take_removed(&mut self.$field, &mut self.bound_globals, name)
                {
                    manager.destroy();
                }
            };
        }

        destroy_removed!(screencopy_manager);
        destroy_removed!(viewporter);
        destroy_removed!(xdg_output_manager);
        destroy_removed!(cursor_shape_manager);

        if let Some(manager) = take_removed(&mut self.layer_shell, &mut self.bound_globals, name)
            && manager.version() >= 3
        {
            manager.destroy();
        }

        if let Some(seat) = take_removed(&mut self.seat, &mut self.bound_globals, name) {
            self.release_pointer();
            self.release_keyboard();
            seat.release();
        }

        // wl_compositor has no destructor at the version that is bound
        drop(take_removed(
            &mut self.compositor,
            &mut self.bound_globals,
            name,
        ));
    }
}

impl Dispatch<WlRegistry, GlobalListContents> for State {
    fn event(
        state: &mut Self,
        registry: &WlRegistry,
        event: <WlRegistry as Proxy>::Event,
        _data: &GlobalListContents,
        _conn: &wayland_client::Connection,
        qh: &QueueHandle<Self>,
    ) {
        match event {
            Event::Global {
                name,
                interface,
                version,
            } => {
                let global = Global {
                    name,
                    interface,
                    version,
                };

                state.globals.push(global.clone());
                state.bind_global(registry, qh, &global);
                state.request_xdg_outputs(qh);
            }
            Event::GlobalRemove { name } => {
                state.remove_global(name);
                state.bind_advertised_globals(registry, qh);
            }
            other => {
                trace!(?other, "ignored registry event");
            }
        }
    }
}

/// Binds one advertised global into the empty slot for its interface.
struct Binder<'a> {
    registry: &'a WlRegistry,
    qh: &'a QueueHandle<State>,
    global: &'a Global,
    bound_globals: &'a mut HashMap<&'static str, u32>,
}

impl Binder<'_> {
    /// Ignores other interfaces, filled slots, and versions below `versions`.
    fn bind<I>(&mut self, slot: &mut Option<I>, versions: RangeInclusive<u32>)
    where
        I: Proxy + 'static,
        State: Dispatch<I, ()>,
    {
        let interface = I::interface();
        let global = self.global;

        if global.interface != interface.name || slot.is_some() {
            return;
        }

        if global.version < *versions.start() {
            warn!(
                interface = interface.name,
                name = global.name,
                advertised = global.version,
                required = *versions.start(),
                "skipping unsupported Wayland global"
            );
            return;
        }

        let version = global.version.min(*versions.end());

        *slot = Some(self.registry.bind(global.name, version, self.qh, ()));
        self.bound_globals.insert(interface.name, global.name);
    }
}

fn take_removed<I: Proxy>(
    slot: &mut Option<I>,
    bound_globals: &mut HashMap<&'static str, u32>,
    name: u32,
) -> Option<I> {
    let interface = I::interface().name;

    if bound_globals.get(interface).copied() != Some(name) {
        return None;
    }

    bound_globals.remove(interface);
    slot.take()
}

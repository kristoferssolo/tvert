use crate::{
    Global, OutputId,
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
    }

    fn bind_global(&mut self, registry: &WlRegistry, qh: &QueueHandle<Self>, global: &Global) {
        if global.interface == WlOutput::interface().name {
            let id = OutputId(global.name);

            if self.output_states.contains_key(&id) {
                return;
            }

            if global.version < 4 {
                warn!(
                    name = global.name,
                    advertised = global.version,
                    required = 4,
                    "skipping output without wl_output name support"
                );
                return;
            }

            let proxy = registry.bind(global.name, 4, qh, OutputData { id });
            self.output_states.insert(id, OutputState::new(proxy));
            return;
        }

        match global.interface.as_str() {
            "wl_shm" => bind_once(
                &mut self.shm,
                &mut self.bound_globals,
                registry,
                global,
                qh,
                1..=2,
            ),
            "ext_output_image_capture_source_manager_v1" => bind_once(
                &mut self.output_capture_source_manager,
                &mut self.bound_globals,
                registry,
                global,
                qh,
                1..=1,
            ),
            "ext_image_copy_capture_manager_v1" => bind_once(
                &mut self.capture_manager,
                &mut self.bound_globals,
                registry,
                global,
                qh,
                1..=1,
            ),
            "zwlr_screencopy_manager_v1" => bind_once(
                &mut self.screencopy_manager,
                &mut self.bound_globals,
                registry,
                global,
                qh,
                1..=3,
            ),
            "wl_compositor" => bind_once(
                &mut self.compositor,
                &mut self.bound_globals,
                registry,
                global,
                qh,
                4..=4,
            ),
            "zwlr_layer_shell_v1" => bind_once(
                &mut self.layer_shell,
                &mut self.bound_globals,
                registry,
                global,
                qh,
                1..=4,
            ),
            "wp_viewporter" => bind_once(
                &mut self.viewporter,
                &mut self.bound_globals,
                registry,
                global,
                qh,
                1..=1,
            ),
            "wl_seat" => bind_once(
                &mut self.seat,
                &mut self.bound_globals,
                registry,
                global,
                qh,
                5..=5,
            ),
            "wp_cursor_shape_manager_v1" => bind_once(
                &mut self.cursor_shape_manager,
                &mut self.bound_globals,
                registry,
                global,
                qh,
                1..=1,
            ),
            _ => {}
        }
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

        if self
            .overlay
            .as_ref()
            .is_some_and(|overlay| overlay.output_id() == id)
        {
            drop(self.overlay.take());
        }

        if self
            .capture_source
            .as_ref()
            .is_some_and(|(output_id, _)| *output_id == id)
            && let Some((_, source)) = self.capture_source.take()
        {
            source.destroy();
        }

        self.outputs.remove(&id);

        if let Some(output) = self.output_states.remove(&id)
            && output.proxy.version() >= 3
        {
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

        destroy_removed!(output_capture_source_manager);
        destroy_removed!(capture_manager);
        destroy_removed!(screencopy_manager);
        destroy_removed!(viewporter);
        destroy_removed!(cursor_shape_manager);

        if let Some(manager) = take_removed(&mut self.layer_shell, &mut self.bound_globals, name)
            && manager.version() >= 3
        {
            manager.destroy();
        }

        if let Some(seat) = take_removed(&mut self.seat, &mut self.bound_globals, name) {
            self.release_pointer();
            seat.release();
        }

        // wl_compositor has no destructor at the version that gets bind
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

fn bind_once<I>(
    slot: &mut Option<I>,
    bound_globals: &mut HashMap<&'static str, u32>,
    registry: &WlRegistry,
    global: &Global,
    qh: &QueueHandle<State>,
    versions: RangeInclusive<u32>,
) where
    I: Proxy + 'static,
    State: Dispatch<I, ()>,
{
    let interface = I::interface();

    if global.interface != interface.name {
        return;
    }

    if slot.is_some() {
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

    *slot = Some(registry.bind(global.name, version, qh, ()));
    bound_globals.insert(interface.name, global.name);
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

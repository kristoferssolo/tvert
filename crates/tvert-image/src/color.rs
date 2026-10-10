use crate::Error;

/// Returns alpha-weighted RGB channels and the original alpha
pub fn weighted_channels([red, green, blue, alpha]: [u8; 4]) -> Result<[u64; 4], Error> {
    let alpha = u64::from(alpha);
    let mut channels = [0; 4];
    for (channel, value) in channels.iter_mut().zip([red, green, blue, 1]) {
        *channel = u64::from(value)
            .checked_mul(alpha)
            .ok_or(Error::SizeOverflow)?;
    }
    Ok(channels)
}

/// Converts alpha-weighted channel totals to a straight RGBA average
pub fn average_color([red, green, blue, alpha]: [u128; 4], count: u128) -> Result<[u8; 4], Error> {
    if alpha == 0 {
        return Ok([0; 4]);
    }
    let mut color = [0; 4];
    for (channel, (sum, divisor)) in
        color
            .iter_mut()
            .zip([(red, alpha), (green, alpha), (blue, alpha), (alpha, count)])
    {
        let value = sum.checked_div(divisor).ok_or(Error::SizeOverflow)?;
        *channel = u8::try_from(value).map_err(|_| Error::SizeOverflow)?;
    }
    Ok(color)
}

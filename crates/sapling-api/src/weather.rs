//! A world's weather.

use crate::ShardCtx;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Weather {
    #[default]
    Clear,
    Rain,
    /// Rain with thunder.
    Thunder,
}

impl ShardCtx<'_> {
    /// The weather of the world in scope.
    pub fn weather(&self) -> Weather {
        self.backend.weather()
    }

    /// Sets the weather of the world in scope for `ticks`, after which it is clear again, or
    /// until it is set anew where `ticks` is `None`. Its players see it at once.
    pub fn set_weather(&mut self, weather: Weather, ticks: Option<u32>) {
        self.backend.set_weather(weather, ticks);
    }
}

/// The weather side of [`crate::Backend`].
pub trait WeatherBackend {
    fn weather(&self) -> Weather;
    fn set_weather(&mut self, weather: Weather, ticks: Option<u32>);
}

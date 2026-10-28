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

    /// Sets the weather of the world in scope for `ticks` (`None`: `/weather`'s own 6000 to 17980),
    /// after which the `doWeatherCycle` rule turns it as BDS does; where the rule is off it stays.
    /// Its players see it at once.
    pub fn set_weather(&mut self, weather: Weather, ticks: Option<u32>) {
        self.backend.set_weather(weather, ticks);
    }
}

/// The weather side of [`crate::Backend`].
pub trait WeatherBackend {
    fn weather(&self) -> Weather;
    fn set_weather(&mut self, weather: Weather, ticks: Option<u32>);
}

#[cfg(test)]
impl WeatherBackend for crate::testing::Fake {
    fn weather(&self) -> Weather {
        Weather::Clear
    }
    fn set_weather(&mut self, _: Weather, _: Option<u32>) {}
}

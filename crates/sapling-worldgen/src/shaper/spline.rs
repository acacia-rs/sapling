//! Java 1.18.1 `CubicSpline` over the terrain-shaper point, in f32.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShaperPoint {
    pub continents: f32,
    pub erosion: f32,
    pub weirdness: f32,
    pub ridges: f32,
}

impl ShaperPoint {
    pub fn new(continents: f32, erosion: f32, weirdness: f32) -> Self {
        Self { continents, erosion, weirdness, ridges: peaks_and_valleys(weirdness) }
    }
}

/// Java `TerrainShaper.peaksAndValleys`.
pub fn peaks_and_valleys(weirdness: f32) -> f32 {
    -((weirdness.abs() - 0.666_666_7).abs() - 0.333_333_34) * 3.0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Coordinate {
    Continents,
    Erosion,
    Weirdness,
    Ridges,
}

impl Coordinate {
    fn of(self, point: &ShaperPoint) -> f32 {
        match self {
            Self::Continents => point.continents,
            Self::Erosion => point.erosion,
            Self::Weirdness => point.weirdness,
            Self::Ridges => point.ridges,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Spline {
    Constant(f32),
    Multipoint { coordinate: Coordinate, points: Vec<SplinePoint> },
}

#[derive(Debug, Clone)]
pub struct SplinePoint {
    pub location: f32,
    pub value: Spline,
    pub derivative: f32,
}

impl Spline {
    pub fn builder(coordinate: Coordinate) -> SplineBuilder {
        SplineBuilder { coordinate, points: Vec::new() }
    }

    pub fn apply(&self, point: &ShaperPoint) -> f32 {
        let (coordinate, points) = match self {
            Self::Constant(value) => return *value,
            Self::Multipoint { coordinate, points } => (coordinate, points),
        };
        let f = coordinate.of(point);
        // Java: binarySearch(n -> f < location[n]) - 1, the last point with location <= f.
        let below = points.iter().take_while(|p| !(f < p.location)).count() as isize - 1;
        let last = points.len() - 1;
        if below < 0 {
            let first = &points[0];
            return first.value.apply(point) + first.derivative * (f - first.location);
        }
        let i = below as usize;
        if i == last {
            let end = &points[last];
            return end.value.apply(point) + end.derivative * (f - end.location);
        }
        let (a, b) = (&points[i], &points[i + 1]);
        let width = b.location - a.location;
        let t = (f - a.location) / width;
        let va = a.value.apply(point);
        let vb = b.value.apply(point);
        let p = a.derivative * width - (vb - va);
        let q = -b.derivative * width + (vb - va);
        lerp(t, va, vb) + t * (1.0 - t) * lerp(t, p, q)
    }
}

pub fn lerp(t: f32, a: f32, b: f32) -> f32 {
    a + t * (b - a)
}

pub struct SplineBuilder {
    coordinate: Coordinate,
    points: Vec<SplinePoint>,
}

impl SplineBuilder {
    pub fn point(mut self, location: f32, value: impl Into<Spline>, derivative: f32) -> Self {
        self.points.push(SplinePoint { location, value: value.into(), derivative });
        self
    }

    pub fn build(self) -> Spline {
        Spline::Multipoint { coordinate: self.coordinate, points: self.points }
    }
}

impl From<f32> for Spline {
    fn from(value: f32) -> Self {
        Self::Constant(value)
    }
}

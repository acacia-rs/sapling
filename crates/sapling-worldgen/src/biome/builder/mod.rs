//! BDS's overworld biome parameter list (148230a70): Java's `OverworldBiomeBuilder` with Bedrock
//! biome ids, in the order BDS adds the entries, which decides how its search tree breaks ties.

mod slices;
mod tables;

use tables::*;

/// A quantized (x10000) parameter range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Range(pub i64, pub i64);

/// Temperature, humidity, continentalness, erosion, depth, weirdness, offset.
pub type Space = [Range; 7];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    pub space: Space,
    pub biome: u32,
}

fn quantize(value: f32) -> i64 {
    (value as f64 * 10000.0) as i64
}

fn span(min: f32, max: f32) -> Range {
    Range(quantize(min), quantize(max))
}

fn join(low: Range, high: Range) -> Range {
    Range(low.0, high.1)
}

fn point(value: f32) -> Range {
    span(value, value)
}

struct Builder {
    entries: Vec<Entry>,
    full: Range,
    t: [Range; 5],
    h: [Range; 5],
    e: [Range; 7],
    c: Continents,
}

impl Builder {
    fn add(&mut self, [t, h, c, e]: [Range; 4], depth: Range, w: Range, biome: Option<u32>) {
        if let Some(biome) = biome {
            self.entries.push(Entry { space: [t, h, c, e, depth, w, point(0.0)], biome });
        }
    }

    /// Java `addSurfaceBiome`: at depth 0 and depth 1.
    fn surface(&mut self, t: Range, h: Range, c: Range, e: Range, w: Range, biome: u32) {
        self.add([t, h, c, e], point(0.0), w, Some(biome));
        self.add([t, h, c, e], point(1.0), w, Some(biome));
    }

    fn off_coast(&mut self) {
        let full = self.full;
        self.surface(full, full, self.c.mushroom, full, full, MUSHROOM_FIELDS);
        for i in 0..5 {
            self.surface(self.t[i], full, self.c.deep_ocean, full, full, OCEANS[0][i]);
            self.surface(self.t[i], full, self.c.ocean, full, full, OCEANS[1][i]);
        }
    }

    fn underground(&mut self) {
        let (full, caves) = (self.full, span(0.2, 0.9));
        self.add([full, full, span(0.8, 1.0), full], caves, full, Some(DRIPSTONE_CAVES));
        self.add([full, span(0.7, 1.0), full, full], caves, full, Some(LUSH_CAVES));
        let sulfur = [full, full, join(self.c.coast, self.c.inland), join(self.e[5], self.e[6])];
        self.add(sulfur, caves, span(-1.1, -0.85), Some(SULFUR_CAVES));
        self.add([full, full, full, join(self.e[0], self.e[1])], point(1.1), full, Some(DEEP_DARK));
    }
}

/// The overworld entries in BDS's order.
pub fn overworld_entries() -> Vec<Entry> {
    let mut builder = Builder {
        entries: Vec::with_capacity(8000),
        full: full(),
        t: temperatures(),
        h: humidities(),
        e: erosions(),
        c: continents(),
    };
    builder.off_coast();
    builder.inland();
    builder.underground();
    builder.entries
}

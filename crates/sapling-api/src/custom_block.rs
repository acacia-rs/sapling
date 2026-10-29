//! Custom blocks ([`crate::Content::block`]): a look (geometry, materials and their textures), states
//! and the permutations they switch on, and how the block stands, lights and breaks.

use crate::content::CreativeCategory;

/// The values a custom block state takes, in the order its permutations are counted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateValues {
    Bool,
    Int(Vec<i32>),
    Text(Vec<String>),
}

impl StateValues {
    pub fn len(&self) -> usize {
        match self {
            Self::Bool => 2,
            Self::Int(values) => values.len(),
            Self::Text(values) => values.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// A box in a block, in pixels: 0 to 16 on each axis, from the block's lowest corner.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlockBox {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

impl BlockBox {
    pub const FULL: Self = Self { min: [0.0; 3], max: [16.0; 3] };

    pub fn new(min: [f32; 3], max: [f32; 3]) -> Self {
        Self { min, max }
    }
}

/// How a material is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RenderMethod {
    #[default]
    Opaque,
    /// Fully see-through where the texture is (leaves, glass panes).
    AlphaTest,
    /// Partly see-through (stained glass).
    Blend,
    DoubleSided,
}

impl RenderMethod {
    pub fn name(self) -> &'static str {
        match self {
            Self::Opaque => "opaque",
            Self::AlphaTest => "alpha_test",
            Self::Blend => "blend",
            Self::DoubleSided => "double_sided",
        }
    }
}

/// What one face (or every face, `*`) of a block is drawn with: a texture of the block's.
#[derive(Debug, Clone, PartialEq)]
pub struct Material {
    /// The name of one of the block's [`CustomBlock::textures`].
    pub texture: String,
    pub render_method: RenderMethod,
    pub face_dimming: bool,
    pub ambient_occlusion: bool,
}

impl Material {
    pub fn new(texture: impl Into<String>) -> Self {
        Self { texture: texture.into(), render_method: RenderMethod::Opaque, face_dimming: true, ambient_occlusion: true }
    }

    pub fn render_method(mut self, method: RenderMethod) -> Self {
        self.render_method = method;
        self.ambient_occlusion = matches!(method, RenderMethod::Opaque | RenderMethod::DoubleSided);
        self
    }
}

/// The block's shape as drawn.
#[derive(Debug, Clone, PartialEq)]
pub enum Geometry {
    /// `minecraft:geometry.full_block`.
    Cube,
    /// A model of the pack's `models/blocks/`: its identifier (`geometry.example.lamp`) and its
    /// `.geo.json`.
    Model { identifier: String, json: Vec<u8> },
}

/// What a custom block leaves when broken.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum BlockDrop {
    /// Its own item.
    #[default]
    Itself,
    Nothing,
    /// These items, each with its count.
    Items(Vec<(String, u8)>),
}

/// Components that apply while a Molang condition over the block's states holds
/// (`q.block_state('example:lit') == true`). Each `Some` replaces the block's own.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Permutation {
    pub condition: String,
    pub geometry: Option<Geometry>,
    pub materials: Option<Vec<(String, Material)>>,
    pub collision: Option<Option<BlockBox>>,
    pub selection: Option<Option<BlockBox>>,
    pub light_emission: Option<u8>,
    /// Turns of 90° about x, y and z.
    pub rotation: Option<[i32; 3]>,
}

impl Permutation {
    pub fn when(condition: impl Into<String>) -> Self {
        Self { condition: condition.into(), ..Self::default() }
    }
}

/// A custom block. Built with [`CustomBlock::new`] and the setters; a full cube of one texture by
/// default, as hard as dirt, dropping itself.
#[derive(Debug, Clone, PartialEq)]
pub struct CustomBlock {
    /// Namespaced, never `minecraft:`.
    pub identifier: String,
    pub display_name: String,
    pub geometry: Geometry,
    /// Face (`*`, `up`, `down`, `north`, `south`, `east`, `west`) to the material drawn there.
    pub materials: Vec<(String, Material)>,
    /// Texture name to its PNG.
    pub textures: Vec<(String, Vec<u8>)>,
    /// State name (namespaced, `example:lit`) to its values.
    pub states: Vec<(String, StateValues)>,
    pub permutations: Vec<Permutation>,
    /// `None`: nothing stops an entity.
    pub collision: Option<BlockBox>,
    /// `None`: it cannot be aimed at.
    pub selection: Option<BlockBox>,
    pub light_emission: u8,
    /// Light levels it takes from light passing through: 15 stops all.
    pub light_dampening: u8,
    /// Seconds to break it by hand; `None`: it cannot be broken (bedrock).
    pub break_seconds: Option<f32>,
    pub friction: f32,
    pub drops: BlockDrop,
    pub category: CreativeCategory,
    pub group: String,
    /// `#rrggbb` on maps.
    pub map_colour: Option<String>,
}

impl CustomBlock {
    /// A full cube drawn with `texture` on every face.
    pub fn new(identifier: impl Into<String>, display_name: impl Into<String>, texture: Vec<u8>) -> Self {
        let identifier = identifier.into();
        let texture_name = identifier.replace(':', "_");
        Self {
            display_name: display_name.into(),
            geometry: Geometry::Cube,
            materials: vec![("*".to_owned(), Material::new(texture_name.clone()))],
            textures: vec![(texture_name, texture)],
            states: Vec::new(),
            permutations: Vec::new(),
            collision: Some(BlockBox::FULL),
            selection: Some(BlockBox::FULL),
            light_emission: 0,
            light_dampening: 15,
            break_seconds: Some(0.75),
            friction: 0.4,
            drops: BlockDrop::Itself,
            category: CreativeCategory::Construction,
            group: String::new(),
            map_colour: None,
            identifier,
        }
    }

    pub fn geometry(mut self, geometry: Geometry) -> Self {
        self.geometry = geometry;
        self
    }

    /// Draws `face` with `material`; `*` is every face without one of its own.
    pub fn material(mut self, face: impl Into<String>, material: Material) -> Self {
        let face = face.into();
        self.materials.retain(|(known, _)| *known != face);
        self.materials.push((face, material));
        self
    }

    /// Adds a texture the materials can name.
    pub fn texture(mut self, name: impl Into<String>, png: Vec<u8>) -> Self {
        let name = name.into();
        self.textures.retain(|(known, _)| *known != name);
        self.textures.push((name, png));
        self
    }

    pub fn state(mut self, name: impl Into<String>, values: StateValues) -> Self {
        self.states.push((name.into(), values));
        self
    }

    pub fn permutation(mut self, permutation: Permutation) -> Self {
        self.permutations.push(permutation);
        self
    }

    pub fn collision(mut self, collision: Option<BlockBox>) -> Self {
        self.collision = collision;
        self
    }

    pub fn selection(mut self, selection: Option<BlockBox>) -> Self {
        self.selection = selection;
        self
    }

    pub fn light_emission(mut self, level: u8) -> Self {
        self.light_emission = level.min(15);
        self
    }

    pub fn light_dampening(mut self, levels: u8) -> Self {
        self.light_dampening = levels.min(15);
        self
    }

    pub fn break_seconds(mut self, seconds: Option<f32>) -> Self {
        self.break_seconds = seconds;
        self
    }

    pub fn friction(mut self, friction: f32) -> Self {
        self.friction = friction;
        self
    }

    pub fn drops(mut self, drops: BlockDrop) -> Self {
        self.drops = drops;
        self
    }

    pub fn category(mut self, category: CreativeCategory) -> Self {
        self.category = category;
        self
    }

    pub fn group(mut self, group: impl Into<String>) -> Self {
        self.group = group.into();
        self
    }

    pub fn map_colour(mut self, colour: impl Into<String>) -> Self {
        self.map_colour = Some(colour.into());
        self
    }

    /// The part of the identifier after the namespace.
    pub fn short_name(&self) -> &str {
        self.identifier.rsplit(':').next().unwrap_or(&self.identifier)
    }

    /// Every combination of its states' values, the first state's changing slowest; one empty
    /// combination for a block without states.
    pub fn permutations_of_states(&self) -> Vec<Vec<(String, StateValue)>> {
        let mut all = vec![Vec::new()];
        for (name, values) in &self.states {
            let values: Vec<StateValue> = match values {
                StateValues::Bool => vec![StateValue::Bool(false), StateValue::Bool(true)],
                StateValues::Int(values) => values.iter().copied().map(StateValue::Int).collect(),
                StateValues::Text(values) => values.iter().cloned().map(StateValue::Text).collect(),
            };
            all = all.into_iter().flat_map(|done: Vec<(String, StateValue)>| {
                values.iter().map(move |value| {
                    let mut next = done.clone();
                    next.push((name.clone(), value.clone()));
                    next
                })
            }).collect();
        }
        all
    }
}

/// One value of a custom block state.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StateValue {
    Bool(bool),
    Int(i32),
    Text(String),
}

//! A player's skin for other clients: the claims of its login turned into the wire form, the way
//! BDS turns them (tests check this against BDS's own output). A client's claims are untrusted and
//! go to every other client, so a skin that does not hold together is replaced by a plain one.

use acacia_auth::login::Skin as Claims;
use acacia_proto::manual::Uuid;
use acacia_proto::types::{PersonaPieceType, Skin, SkinAnimationsItem, SkinArmSize, SkinImage, SkinPersonalPiecesItem, SkinPieceTintColorsItem};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use bytes::Bytes;
use serde_json::Value;

/// Persona skins are 256 px; nothing vanilla is larger than 512.
const MAX_SIDE: u32 = 512;
const MAX_TEXT: usize = 1 << 20;
const MAX_ID: usize = 256;
const MAX_ANIMATIONS: usize = 8;
const MAX_PIECES: usize = 64;

const PIECES: [(&str, PersonaPieceType); 27] = {
    use PersonaPieceType as P;
    [
        ("skeleton", P::Skeleton),
        ("body", P::Body),
        ("skin", P::Skin),
        ("bottom", P::Bottom),
        ("feet", P::Feet),
        ("dress", P::Dress),
        ("top", P::Top),
        ("high_pants", P::HighPants),
        ("hands", P::Hands),
        ("outerwear", P::Outerwear),
        ("facial_hair", P::FacialHair),
        ("mouth", P::Mouth),
        ("eyes", P::Eyes),
        ("hair", P::Hair),
        ("hood", P::Hood),
        ("back", P::Back),
        ("face_accessory", P::FaceAccessory),
        ("head", P::Head),
        ("legs", P::Legs),
        ("left_leg", P::LeftLeg),
        ("right_leg", P::RightLeg),
        ("arms", P::Arms),
        ("left_arm", P::LeftArm),
        ("right_arm", P::RightArm),
        ("capes", P::Capes),
        ("classic_skin", P::ClassicSkin),
        ("emote", P::Emote),
    ]
};

/// The skin other clients are sent for a login's claims.
pub(crate) fn wire_skin(claims: &Claims) -> Skin {
    convert(claims).unwrap_or_else(|| {
        tracing::debug!(skin = %claims.skin_id.chars().take(64).collect::<String>(), "replacing a malformed skin");
        plain()
    })
}

fn image(width: u32, height: u32, base64: &str) -> Option<SkinImage> {
    let data = STANDARD.decode(base64).ok()?;
    let fits = width <= MAX_SIDE && height <= MAX_SIDE && data.len() == (width * height * 4) as usize;
    fits.then(|| SkinImage { width: width as i32, height: height as i32, data: Bytes::from(data) })
}

fn text(base64: &str) -> Option<String> {
    let text = String::from_utf8(STANDARD.decode(base64).ok()?).ok()?;
    (text.len() <= MAX_TEXT).then_some(text)
}

fn id(text: &str) -> Option<String> {
    (text.len() <= MAX_ID && !text.chars().any(char::is_control)).then(|| text.to_owned())
}

/// JSON as BDS forwards geometry: parsed and written back without whitespace, keys sorted
/// (BDS pads fractions with zeros; the values are the same).
fn compact_json(text: &str) -> Option<String> {
    if text.trim().is_empty() {
        return Some(String::new());
    }
    serde_json::from_str::<Value>(text).ok().map(|json| json.to_string())
}

/// `#aarrggbb` as the wire's integer, whose bytes in order are blue, green, red, alpha.
fn colour(text: &str) -> i32 {
    u32::from_str_radix(text.trim_start_matches('#'), 16).map_or(0, |argb| argb.swap_bytes() as i32)
}

fn piece_name(claimed: &str) -> &str {
    claimed.strip_prefix("persona_").unwrap_or(claimed)
}

fn animation(value: &Value) -> Option<SkinAnimationsItem> {
    let number = |key: &str| value.get(key).and_then(Value::as_f64);
    let side = |key: &str| number(key).filter(|n| (0.0..=f64::from(MAX_SIDE)).contains(n)).map(|n| n as u32);
    Some(SkinAnimationsItem {
        skin_image: image(side("ImageWidth")?, side("ImageHeight")?, value.get("Image")?.as_str()?)?,
        animation_type: number("Type")? as u32,
        animation_frames: number("Frames")? as f32,
        expression_type: number("AnimationExpression")? as u32,
    })
}

fn piece(value: &Value) -> Option<SkinPersonalPiecesItem> {
    let field = |key: &str| value.get(key).and_then(Value::as_str);
    let name = piece_name(field("PieceType")?);
    Some(SkinPersonalPiecesItem {
        piece_id: id(field("PieceId")?)?,
        piece_type: PIECES.iter().find(|(known, _)| *known == name)?.1,
        pack_id: field("PackId")?.parse::<Uuid>().ok()?,
        is_default_piece: value.get("IsDefault")?.as_bool()?,
        product_id: id(field("ProductId")?)?,
    })
}

fn tint(value: &Value) -> Option<SkinPieceTintColorsItem> {
    let claimed: Vec<&str> = value.get("Colors")?.as_array()?.iter().map(Value::as_str).collect::<Option<_>>()?;
    let colors: [i32; 4] = claimed.iter().map(|c| colour(c)).collect::<Vec<_>>().try_into().ok()?;
    Some(SkinPieceTintColorsItem { piece_type: id(piece_name(value.get("PieceType")?.as_str()?))?, colors })
}

fn convert(claims: &Claims) -> Option<Skin> {
    let skin_data = image(claims.skin_image_width, claims.skin_image_height, &claims.skin_data).filter(|i| !i.data.is_empty())?;
    let counts_fit = claims.animated_image_data.len() <= MAX_ANIMATIONS && claims.persona_pieces.len() <= MAX_PIECES && claims.piece_tint_colours.len() <= MAX_PIECES;
    if !counts_fit || claims.skin_id.is_empty() {
        return None;
    }
    let skin_id = id(&claims.skin_id)?;
    Some(Skin {
        full_skin_id: skin_id.clone(),
        skin_id,
        play_fab_id: String::new(),
        skin_resource_pack: text(&claims.skin_resource_patch).filter(|patch| serde_json::from_str::<Value>(patch).is_ok())?,
        skin_data,
        animations: claims.animated_image_data.iter().map(animation).collect::<Option<_>>()?,
        cape_data: image(claims.cape_image_width, claims.cape_image_height, &claims.cape_data)?,
        geometry_data: compact_json(&text(&claims.skin_geometry)?)?,
        geometry_data_version: id(&text(&claims.skin_geometry_version)?)?,
        animation_data: text(&claims.skin_animation_data)?,
        cape_id: id(&claims.cape_id)?,
        arm_size: if claims.arm_size == "slim" { SkinArmSize::Slim } else { SkinArmSize::Wide },
        skin_color: colour(&claims.skin_colour),
        personal_pieces: claims.persona_pieces.iter().map(piece).collect::<Option<_>>()?,
        piece_tint_colors: claims.piece_tint_colours.iter().map(tint).collect::<Option<_>>()?,
        premium: claims.premium_skin,
        persona: claims.persona_skin,
        cape_on_classic: claims.cape_on_classic_skin,
        primary_user: false,
        overriding_player_appearance: claims.override_skin,
        trusted: trusted(claims.trusted_skin),
        profile_hash: String::new(),
    })
}

fn trusted(claimed: bool) -> String {
    if claimed { "true" } else { "false" }.to_owned()
}

/// A one-colour 64x64 skin on the client's built-in player model.
fn plain() -> Skin {
    const SIDE: i32 = 64;
    const GREY: [u8; 4] = [0x9a, 0x9a, 0x9a, 0xff];
    let empty = || SkinImage { width: 0, height: 0, data: Bytes::new() };
    Skin {
        skin_id: "sapling-plain".into(),
        play_fab_id: String::new(),
        skin_resource_pack: r#"{"geometry":{"default":"geometry.humanoid.custom"}}"#.into(),
        skin_data: SkinImage { width: SIDE, height: SIDE, data: Bytes::from(GREY.repeat((SIDE * SIDE) as usize)) },
        animations: Vec::new(),
        cape_data: empty(),
        geometry_data: String::new(),
        geometry_data_version: "0.0.0".into(),
        animation_data: String::new(),
        cape_id: String::new(),
        full_skin_id: "sapling-plain".into(),
        arm_size: SkinArmSize::Wide,
        skin_color: 0,
        personal_pieces: Vec::new(),
        piece_tint_colors: Vec::new(),
        premium: false,
        persona: false,
        cape_on_classic: false,
        primary_user: false,
        overriding_player_appearance: false,
        trusted: trusted(true),
        profile_hash: String::new(),
    }
}

#[cfg(test)]
mod tests;

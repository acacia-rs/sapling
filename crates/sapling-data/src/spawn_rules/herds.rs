//! The events a spawn rule's `minecraft:herd` fires on the mobs of a herd: BDS picks one of a
//! condition's herds at random (143907530) and, in the order they spawned, fires its
//! `initial_event` on the first `initial_event_count` and its `event` on all from the
//! `event_skip_count`th on (1415ed0d0). So a fox herd's first two are grown and the rest babies.

/// One herd of a condition: `herds=<event>@<skip>/<initial>@<count>|...` in spawn-rules.tsv.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HerdEvents {
    event: Option<&'static str>,
    skip: u32,
    initial: Option<&'static str>,
    initial_count: u32,
}

fn named(text: &'static str) -> Option<(Option<&'static str>, u32)> {
    let (event, count) = text.split_once('@')?;
    Some(((!event.is_empty()).then_some(event), count.parse().ok()?))
}

impl HerdEvents {
    pub(super) fn parse_all(value: &'static str) -> Vec<HerdEvents> {
        value
            .split('|')
            .filter_map(|herd| {
                let (event, initial) = herd.split_once('/')?;
                let ((event, skip), (initial, initial_count)) = (named(event)?, named(initial)?);
                Some(HerdEvents { event, skip, initial, initial_count })
            })
            .collect()
    }

    /// The events fired on the herd's `member`th mob (from 0), the initial one first.
    pub fn events_of(&self, member: u32) -> impl Iterator<Item = &'static str> {
        let initial = self.initial.filter(|_| member < self.initial_count);
        let event = self.event.filter(|_| member >= self.skip);
        initial.into_iter().chain(event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_herd_fires_its_event_past_the_skipped_and_its_initial_on_the_first() {
        let [fox] = HerdEvents::parse_all("minecraft:entity_born@2/@0").try_into().unwrap();
        let fired: Vec<Vec<&str>> = (0..4).map(|member| fox.events_of(member).collect()).collect();
        assert_eq!(fired, [vec![], vec![], vec!["minecraft:entity_born"], vec!["minecraft:entity_born"]]);
        let [patrol] = HerdEvents::parse_all("minecraft:follow@1/minecraft:captain@1").try_into().unwrap();
        assert_eq!(patrol.events_of(0).collect::<Vec<_>>(), ["minecraft:captain"]);
        assert_eq!(patrol.events_of(1).collect::<Vec<_>>(), ["minecraft:follow"]);
        assert_eq!(HerdEvents::parse_all("a@0/@0|b@0/@0").len(), 2);
    }
}

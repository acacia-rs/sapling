//! NetherNet offers: answered before their connection opens, their signing key kept for its Login.

use std::net::SocketAddr;
use std::time::Instant;

use acacia_nethernet::host::Answer;

use super::Sessions;
use crate::links::{Links, Refusal};

impl Sessions {
    /// Answers a NetherNet offer that reached us at `local`, if its identity is good enough for
    /// this server: online mode wants the auth service's token, offline takes any or none.
    pub fn admit_offer(&mut self, links: &mut Links, offer: &str, local: SocketAddr, trickle: bool, now: Instant) -> Result<Answer, Refusal> {
        let host = links.nethernet.as_mut().ok_or(Refusal::Full)?;
        let identity = acacia_nethernet::verify_offer(offer).map_err(|error| {
            tracing::debug!(%error, "offer refused");
            Refusal::Identity
        })?;
        if self.cfg.online_mode && identity.as_ref().is_none_or(|identity| identity.self_signed) {
            return Err(Refusal::Identity);
        }
        let answered = if trickle { host.trickle_answer(offer, local, now) } else { host.answer(offer, local, now) };
        let answer = answered.map_err(|error| match error {
            acacia_nethernet::Error::HostFull => Refusal::Full,
            error => {
                tracing::debug!(%error, "offer refused");
                Refusal::Offer
            }
        })?;
        if let Some(identity) = identity {
            self.offers.insert(answer.peer, identity.key);
        }
        Ok(answer)
    }
}

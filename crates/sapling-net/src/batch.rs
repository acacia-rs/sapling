//! Encoded batches cut down to their length before they are queued.

use bytes::Bytes;

/// `batch` in an allocation no bigger than a quarter over its length, copying it when needed.
///
/// acacia-session's `put_body` reserves `plain * 9 / 8 + 16` bytes for deflate's output and
/// `freeze` keeps that capacity, so a compressed chunk batch holds several times its length, and
/// the transport keeps the whole allocation until the batch's last fragment is acked.
pub(crate) fn trimmed(batch: Bytes) -> Bytes {
    match batch.try_into_mut() {
        Ok(buf) if buf.capacity() - buf.len() > buf.len() / 4 => Bytes::copy_from_slice(&buf),
        Ok(buf) => buf.freeze(),
        Err(shared) => shared,
    }
}

#[cfg(test)]
mod tests {
    use bytes::{BufMut, BytesMut};

    use super::*;

    fn frozen(len: usize, capacity: usize) -> Bytes {
        let mut buf = BytesMut::with_capacity(capacity);
        buf.put_bytes(7, len);
        buf.freeze()
    }

    fn capacity(batch: Bytes) -> usize {
        batch.try_into_mut().expect("unique").capacity()
    }

    #[test]
    fn spare_capacity_is_dropped_and_the_bytes_kept() {
        let batch = trimmed(frozen(1000, 9000));
        assert_eq!(batch, vec![7u8; 1000]);
        assert_eq!(capacity(batch), 1000);
    }

    #[test]
    fn a_batch_near_its_length_is_not_copied() {
        let batch = frozen(1000, 1200);
        let ptr = batch.as_ptr();
        let batch = trimmed(batch);
        assert_eq!(batch.as_ptr(), ptr);
    }

    #[test]
    fn a_shared_batch_is_left_alone() {
        let batch = frozen(1000, 9000);
        let other = batch.clone();
        assert_eq!(trimmed(batch).as_ptr(), other.as_ptr());
    }
}

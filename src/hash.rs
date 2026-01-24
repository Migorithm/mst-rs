use std::ops::{Deref, DerefMut};

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct NodeHash(pub [u8; 32]);
impl From<[u8; 32]> for NodeHash {
    fn from(value: [u8; 32]) -> Self {
        NodeHash(value)
    }
}
impl Deref for NodeHash {
    type Target = [u8; 32];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for NodeHash {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl NodeHash {
    #[inline]
    pub fn xor(&mut self, source: &NodeHash) {
        for (t, s) in self.iter_mut().zip(source.iter()) {
            *t ^= s;
        }
    }
}

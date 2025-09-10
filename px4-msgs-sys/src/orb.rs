include!(concat!(env!("OUT_DIR"), "/orb.rs"));

impl PartialEq for orb_metadata {
    fn eq(&self, other: &Self) -> bool {
        self.o_id == other.o_id
    }
}

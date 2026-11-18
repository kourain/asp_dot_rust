use bitflags::bitflags;

bitflags! {
    pub(crate) struct DIPropOption: u8 {
        const NONE             = 0b0;
        const DEFAULT          = 0b1;
    }
}

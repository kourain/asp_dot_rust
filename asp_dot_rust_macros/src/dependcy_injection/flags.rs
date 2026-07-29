use bitflags::bitflags;

bitflags! {
    pub(crate) struct InjectFlags: u8 {
        const NONE             = 0b0;
        const CONFIG           = 0b1;
        const SERVICE          = 0b10;
        const INJECT_CONTROLLER = 0b100;
    }
}
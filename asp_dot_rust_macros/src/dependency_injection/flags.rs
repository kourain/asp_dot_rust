use bitflags::bitflags;

bitflags! {
    pub(crate) struct DIPropOption: u8 {
        const NONE             = 0b0;
        const DEFAULT          = 0b1;
    }
}

bitflags! {
    pub(super) struct ConstructorInjectOptions: u8 {
        const OPTIONAL              = 0b0001;
        const ALLOW_ASYNC           = 0b0010;
        const CHECK_REGISTRATIONS   = 0b0100;
        const REGISTER_DEPENDENCIES = 0b1000;
    }
}

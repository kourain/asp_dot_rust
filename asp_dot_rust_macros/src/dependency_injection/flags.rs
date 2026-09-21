use bitflags::bitflags;

bitflags! {
    pub(crate) struct InjectFlags: u8 {
        const NONE             = 0b0;
        const INJECT_CONTROLLER = 0b1;
    }
}

#[derive(PartialEq)]
pub enum HttpInjectType {
    None,
    ShareMutPtr,
    BorrowHttpContext,
    MoveHttpContext,
}

bitflags! {
    pub(crate) struct DIPropOption: u8 {
        const NONE             = 0b0;
        const DEFAULT          = 0b1;
    }
}

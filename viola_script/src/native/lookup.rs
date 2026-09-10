use crate::native::{NativeId, NativeSig};

pub fn lookup_native(command: &str, method: Option<&str>) -> Option<NativeSig> {
    match (command, method) {
        // basic message sender
        ("send", Some("text")) => Some(NativeSig::with_optional(NativeId::SendText, 1, 1)),
        ("send", Some("reaction")) => Some(NativeSig::fixed(NativeId::SendReaction, 1)),

        // interactive message sender
        ("send", Some("single_select")) => Some(NativeSig::fixed(NativeId::SendSingleSelect, 1)),

        // args
        ("args", Some("get")) => Some(NativeSig::fixed(NativeId::ArgsGet, 1)),
        ("args", Some("all")) => Some(NativeSig::fixed(NativeId::ArgsAll, 0)),
        ("args", Some("count")) => Some(NativeSig::fixed(NativeId::ArgsCount, 0)),

        // message info
        ("message", Some("text")) => Some(NativeSig::fixed(NativeId::MessageText, 0)),
        ("message", Some("sender")) => Some(NativeSig::fixed(NativeId::MessageSender, 0)),
        ("message", Some("is_group")) => Some(NativeSig::fixed(NativeId::MessageIsGroup, 0)),
        _ => None,
    }
}

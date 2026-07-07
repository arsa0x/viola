use whatsapp_rust::{proto_helpers::MessageExt, waproto::whatsapp};

pub fn get_text_content(msg: &whatsapp::Message) -> Option<&str> {
    if let Some(text) = msg.text_content() {
        return Some(text);
    }
    if let Some(ext) = &msg.extended_text_message.text {
        return Some(ext);
    }
    if let Some(img) = &msg.image_message.caption {
        return Some(img);
    }
    if let Some(vid) = &msg.video_message.caption {
        return Some(vid);
    }
    if let Some(doc) = &msg.document_message.caption {
        return Some(doc);
    }

    if let Some(onc) = get_text_content(&msg.view_once_message.message) {
        return Some(onc);
    }
    if let Some(onc_v2) = get_text_content(&msg.view_once_message_v2.message) {
        return Some(onc_v2);
    }
    return None;
}

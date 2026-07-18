use whatsapp_rust::{buffa::MessageField, waproto::whatsapp};

#[linkme::distributed_slice(viola_core::command::COMMANDS)]
static TEST: viola_core::Command = viola_core::Command {
    category: "tools",
    group_only: false,
    owner_only: false,
    help: None,
    description: None,
    name: "test",
    triggers: &["test"],
    execute: |ctx: viola_core::Context| {
        Box::pin(async move {
            let card_1 = whatsapp::message::InteractiveMessage {
                header: MessageField::some(whatsapp::message::interactive_message::Header {
                    title: Some("Card 1 Title".into()),
                    ..Default::default()
                }),
                body: MessageField::some(whatsapp::message::interactive_message::Body {
                    text: Some("Description for card 1".into()),
                }),
                ..Default::default()
            };

            let card_2 = whatsapp::message::InteractiveMessage {
                header: MessageField::some(whatsapp::message::interactive_message::Header {
                    title: Some("Card 2 Title".into()),
                    ..Default::default()
                }),
                body: MessageField::some(whatsapp::message::interactive_message::Body {
                    text: Some("Description for card 2".into()),
                }),
                ..Default::default()
            };

            ctx.send()
          .raw(whatsapp::Message {
              interactive_message: MessageField::some(whatsapp::message::InteractiveMessage {
                  header: MessageField::some(whatsapp::message::interactive_message::Header {
                      title: Some("Title".into()),
                      subtitle: Some("Subtitle".into()),
                      // has_media_attachment: (),
                      // bloks_widget: (),
                      // media: (),
                      ..Default::default()
                  }),
                  body: MessageField::some(whatsapp::message::interactive_message::Body {
                      text: Some("Body".into()),
                  }),
                  footer: MessageField::some(whatsapp::message::interactive_message::Footer {
                      text: Some("Footer".into()),
                      // has_media_attachment: (),
                      // media: (),
                      ..Default::default()
                  }),
                  // bloks_widget: (),
                  // context_info: (),
                  // url_tracking_map: (),
                  interactive_message: Some(
                      // pub struct CarouselMessage {
                      //     /// Field 1: `cards`
                      //     pub cards: ::buffa::alloc::vec::Vec<
                      //         super::super::message::InteractiveMessage,
                      //     >,
                      //     /// Field 2: `message_version`
                      //     pub message_version: ::core::option::Option<i32>,
                      //     /// Field 3: `carousel_card_type`
                      //     pub carousel_card_type: ::core::option::Option<
                      //         super::super::message::interactive_message::carousel_message::CarouselCardType,
                      //     >,
                      // }
                      whatsapp::message::interactive_message::InteractiveMessage::CarouselMessage(
                          Box::new(whatsapp::message::interactive_message::CarouselMessage {
                              cards: vec![card_1, card_2],
                              // message_version: (),
                              // carousel_card_type: (),
                              ..Default::default()
                          }),
                      ),
                  ),
                  ..Default::default()
              }),
              ..Default::default()
          })
          .await
        })
    },
};

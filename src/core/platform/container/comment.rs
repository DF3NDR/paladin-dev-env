// TODO Implement this properly

// use crate::core::base::entity::message::{Message, SourceType, DestinationType};
// use crate::core::platform::container::user::User;

// #[derive(Debug, Clone)]
// pub struct UserCommunication {
//     pub subject: String,
//     pub body: String,
// }

// pub type UserCommunicationMessage = Message<UserCommunication>;

// impl UserCommunicationMessage {
//     pub fn new(
//         sender: User,
//         source: String,
//         source_type: SourceType,
//         receiver: User,
//         destination: User,
//         destination_type: DestinationType,
//         subject: String,
//         body: String,
//     ) -> Self {
//         Self {
//             sender,
//             source,
//             source_type,
//             receiver,
//             destination,
//             destination_type,
//             timestamp: Utc::now(),
//             content: UserCommunication { subject, body },
//         }
//     }
// }

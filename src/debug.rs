use vulkano::instance::debug::{
    DebugUtilsMessengerCreateInfo, DebugUtilsMessageSeverity, DebugUtilsMessageType, DebugUtilsMessengerCallback,
    DebugUtilsMessengerCallbackData
};

pub fn debug_printing_messenger() -> DebugUtilsMessengerCreateInfo {
    let print = |_, _, callback_data: DebugUtilsMessengerCallbackData| {
        eprintln!("{:?}", callback_data.message);
    };
    let message_severity = DebugUtilsMessageSeverity::ERROR
        | DebugUtilsMessageSeverity::WARNING;
        // | DebugUtilsMessageSeverity::INFO;
    let message_type = DebugUtilsMessageType::GENERAL
        | DebugUtilsMessageType::VALIDATION
        | DebugUtilsMessageType::PERFORMANCE;
    let user_callback = unsafe { DebugUtilsMessengerCallback::new(print) };
    DebugUtilsMessengerCreateInfo {
        message_severity,
        message_type,
        ..DebugUtilsMessengerCreateInfo::user_callback(user_callback)
    }
}
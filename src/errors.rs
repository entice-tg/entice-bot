error_chain! {
    errors {
        AlreadyStarted
        AlreadyStopped
    }

    foreign_links {
        ConfigError(::config::ConfigError);
        TelebotError(::telebot::Error);
        DatabaseError(::diesel::result::Error);
        TemplateError(::handlebars::TemplateError);
    }
}

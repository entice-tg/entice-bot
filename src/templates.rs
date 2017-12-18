use errors::*;

use handlebars::Handlebars;

pub const JOIN: &'static str = "join";
const TPL_JOIN: &'static str =
    "Hey! I'm @{{username}}.\n\n\
     \
     Now that I'm here, anyone can invite friends to this group by mentioning \
     me in your conversations.";

pub const REPLY_START: &'static str = "reply_start";
const TPL_REPLY_START: &'static str =
    "Hello! I'm @{{username}}.\n\n\
     \
     I help manage inviting new users to groups. If you'd like to use me in \
     your groups, add me as an administrator to get started!";

pub fn register_all(handlebars: &mut Handlebars) -> Result<()> {
    handlebars.register_template_string(REPLY_START, TPL_REPLY_START)?;
    handlebars.register_template_string(JOIN, TPL_JOIN)?;

    Ok(())
}

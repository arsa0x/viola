use crate::{
    error::NativeError,
    native::{ExecContext, Host, Value, specs::SingleSelectSpec, utils},
};

pub async fn send_text<H: Host>(
    args: &[Value],
    ctx: &ExecContext<H>,
) -> Result<Value, NativeError> {
    let text = match args.first() {
        Some(Value::Str(s)) => s.clone(),
        Some(v) => {
            return Err(NativeError::invalid_arg(format!(
                "`:send .text` need str, get {}",
                v.type_name()
            )));
        }
        None => {
            return Err(NativeError::invalid_arg(
                "`:send .text` requires 1 argument",
            ));
        }
    };

    let quoted = match args.get(1) {
        Some(opts @ Value::Object(_)) => utils::get_bool_opt(opts, "quoted", false)?,
        Some(v) => {
            return Err(NativeError::invalid_arg(format!(
                "`:send.text` options must be an object, got {}",
                v.type_name()
            )));
        }
        None => false,
    };

    ctx.host.send_text(&text, quoted).await?;

    Ok(Value::Nil)
}

pub async fn send_single_select<H: Host>(
    args: &[Value],
    ctx: &ExecContext<H>,
) -> Result<Value, NativeError> {
    let spec_obj = match args.first() {
        Some(v @ Value::Object(_)) => v,
        Some(v) => {
            return Err(NativeError::invalid_arg(format!(
                "`:send.single_select` needs an object, got {}",
                v.type_name()
            )));
        }
        None => {
            return Err(NativeError::invalid_arg(
                "`:send.single_select` requires 1 argument",
            ));
        }
    };

    let spec = SingleSelectSpec {
        sections: utils::get_array(spec_obj, "sections")?,
        title: utils::get_str_opt(spec_obj, "title")?,
        text_body: utils::get_str_opt(spec_obj, "text_body")?,
        footer: utils::get_str_opt(spec_obj, "footer")?,
        select_label: utils::get_str_opt(spec_obj, "select_label")?,
        quoted: utils::get_bool_opt(spec_obj, "quoted", false)?,
    };

    ctx.host.send_single_select(spec).await?;
    Ok(Value::Nil)
}

pub async fn send_reaction<H: Host>(
    args: &[Value],
    ctx: &ExecContext<H>,
) -> Result<Value, NativeError> {
    let emoji = match args.first() {
        Some(Value::Str(s)) => s.clone(),
        Some(v) => {
            return Err(NativeError::invalid_arg(format!(
                "`:send.reaction` needs str, got {}",
                v.type_name()
            )));
        }

        None => {
            return Err(NativeError::invalid_arg(
                "`:send.reaction` requires 1 argument",
            ));
        }
    };

    ctx.host.send_reaction(&emoji).await?;

    Ok(Value::Nil)
}

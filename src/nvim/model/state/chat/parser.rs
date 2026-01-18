use super::*;

pub(super) const TAG_CHAT: &'static str = "CHAT";
const TAG_MESSAGE: &'static str = "MESSAGE";
const TAG_TOOL_CALL: &'static str = "TOOLCALL";

pub(super) fn is_self_tag_line(line: &String, tag: &'static str) -> bool {
    line.starts_with(&format!("<{tag}")) && line.ends_with("/>")
}
pub(super) fn is_open_tag_line(line: &String, tag: &'static str) -> bool {
    line.starts_with(&format!("<{tag}")) && line.ends_with(">")
}
pub(super) fn is_close_tag_line(line: &String, tag: &'static str) -> bool {
    line == &format!("</{tag}>")
}
pub(super) fn option_to_arg<T: ToString>(value: &Option<T>) -> String {
    value
        .as_ref()
        .map_or("".to_string(), |n| n.to_string())
}
const ESCAPE: char = '\\';
pub(super) fn escape_quote_arg(value: String) -> String {
    let acc = String::with_capacity(value.len());
    value.chars().fold(acc, |mut acc, c| {
        match c {
            c if matches!(c, ESCAPE | '"') => {
                acc.push(ESCAPE);
                acc.push(c)
            }
            c => acc.push(c),
        }
        acc
    })
}
pub(super) fn unescape_quote_arg(value: &String) -> String {
    let acc = String::with_capacity(value.len());
    let mut last_is_escape_char = false;
    value.chars().fold(acc, |mut acc, c| {
        if last_is_escape_char {
            if matches!(c, ESCAPE | '"') {
                acc.push(c)
            } else {
                // Act like it was, previously, not escaped correctly
                acc.push(ESCAPE);
                acc.push(c)
            }
            last_is_escape_char = false;
        } else {
            match c {
                ESCAPE => last_is_escape_char = true,
                c => acc.push(c),
            }
        }
        acc
    })
}

pub(super) fn write_arg(args: &mut String, key: &str, value: impl std::fmt::Display) {
    let value = escape_quote_arg(value.to_string());
    args.push_str(&format!(r#" {key}="{value}""#));
}
pub(super) fn build_tag_message_lines(message: MessageState) -> Vec<String> {
    let MessageState {
        model,
        status,
        usage,
        mode,
        message:
            mistral::model::Message {
                role,
                name,
                tool_call_id,
                content,
                tool_calls,
                ..
            },
        params: mistral::model::completion::CompletionParams {
            min_tokens, max_tokens, ..
        },
        ..
    } = message;
    let args = &mut String::new();
    write_arg(args, "role", role);
    write_arg(args, "model", model);
    write_arg(args, "status", status);
    write_arg(args, "usage", usage);
    write_arg(args, "mode", mode);
    if let Some(name) = name {
        write_arg(args, "name", name);
    }
    if let Some(tool_call_id) = tool_call_id {
        write_arg(args, "tool_call_id", tool_call_id);
    }
    if let Some(min_tokens) = min_tokens {
        write_arg(args, "min_tokens", min_tokens);
    }
    if let Some(max_tokens) = max_tokens {
        write_arg(args, "max_tokens", max_tokens);
    }
    let mut lines = vec!["".to_string(), "".to_string(), format!(r#"<{TAG_MESSAGE}{args}/>"#)];
    if let Some(tool_calls) = tool_calls {
        lines.extend(
            tool_calls
                .into_iter()
                .flat_map(|tc| build_tag_tool_call_lines(&tc)),
        );
    }
    lines.extend(content.split('\n').map(|s| s.to_string()));
    lines
}
pub(super) fn build_tag_tool_call_lines(tool_call: &mistral::model::ToolCall) -> Vec<String> {
    let mistral::model::ToolCall { function, id, index } = tool_call;
    let args = &mut String::new();
    if let Some(id) = id {
        write_arg(args, "id", id);
    }
    if let Some(index) = index {
        write_arg(args, "index", index);
    }
    write_arg(args, "name", &function.name);
    // write_arg(args, "arguments", &function.arguments);

    // Most of the time function arguments takes only one line.
    let mut lines = Vec::with_capacity(6);
    lines.extend([
        "".to_string(),
        format!("<{TAG_TOOL_CALL}{args}>"),
        "".to_string(),
        "```json".to_string(),
    ]);
    lines.extend(function.arguments.split('\n').map(|s| s.to_string()));
    lines.extend(["```".to_string(), format!("</{TAG_TOOL_CALL}>")]);
    lines
}

pub(super) struct GeneratorArgs {
    pub start: (usize, Row),
    pub end: (usize, Row),
    pub from_begin: bool,
    pub until_end: bool,
    pub rows_range: RowRange,
    pub current_len: usize,
}
impl GeneratorArgs {
    pub(super) fn new(
        positions: &MessagesPositions,
        rows_range: RowRange,
        buffer: &api::Buffer,
    ) -> crate::Result<Self> {
        let buf_last_row = Row::buf_last_row(buffer)?;
        let current_len = positions.0.len();
        let mut it_enum = positions.0.iter().enumerate();
        let mut start = it_enum
            .clone()
            .find(|(_, r)| r.start <= rows_range.start && rows_range.start <= r.end)
            .map(|(i, r)| (i, r.start));
        // .unwrap_or((0, Row::MIN));
        let mut end = it_enum
            .find(|(_, r)| r.start <= rows_range.end && rows_range.end <= r.end)
            .map(|(i, r)| (i, r.end));
        // .unwrap_or((current_len - 1, Row::MAX));
        if start.is_none() || end.is_none() {
            start = Some((0, Row::MIN));
            end = Some((current_len - 1, Row::MAX));
        }
        let start = start.unwrap();
        let end = end.unwrap();
        let from_begin = start.1 == Row::MIN;
        let until_end = end.1 == Row::MAX;
        let until_end = until_end || buf_last_row <= end.1;
        bar::StatusLineChatCache::outdate_messages(buffer, start.0..=end.0);
        Ok(Self {
            start,
            end,
            from_begin,
            until_end,
            current_len,
            rows_range,
        })
    }
    #[inline]
    fn is_full_range(&self) -> bool {
        self.from_begin && self.until_end
    }
    #[inline]
    fn row_end(&self) -> Row {
        self.end.1
    }
}

pub(super) trait Generator {
    type Item;

    fn state(&mut self) -> &mut GeneratorState;
    fn next_line_state(&mut self, line_nb: &Row, line: &String) -> crate::Result<GeneratorState>;
    fn next_line(&mut self, line_nb: &Row, line: &String) -> crate::Result<GeneratorState> {
        let new_state = self.next_line_state(line_nb, line)?;
        *self.state() = new_state;
        Ok(new_state)
    }
    fn next_line_option(&mut self, next_line: &Option<(Row, String)>) -> crate::Result<GeneratorState> {
        match self.state() {
            GeneratorState::Empty | GeneratorState::TagOpened | GeneratorState::TagClosed => match next_line {
                Some((line_nb, line)) => self.next_line(line_nb, line),
                None => Ok(self.state().clone()),
            },
            GeneratorState::Completed => Err("Reused generator.".into_error()),
        }
    }
    fn args(&mut self) -> &GeneratorArgs;
    fn take(&mut self) -> Vec<Self::Item>;
    fn finalise(&mut self) -> crate::Result<()>;
    fn replace(&mut self, target: &mut Vec<Self::Item>, shift: usize) -> crate::Result<()> {
        self.finalise()?;
        let items = self.take();
        let args = self.args();
        if args.is_full_range() {
            *target = items;
        } else {
            let mut it_current_items = std::mem::take(target).into_iter().enumerate();
            let index_start = args.start.0.saturating_sub(shift);
            let index_end = args.end.0.saturating_sub(shift);
            let len_expected = index_start + (args.current_len.saturating_sub(shift) - index_end - 1) + items.len();
            let mut new_items = Vec::<_>::with_capacity(len_expected);
            while let Some((index, msg)) = it_current_items.next() {
                if index >= index_start {
                    break;
                }
                new_items.push(msg);
            }
            new_items.extend(items);
            while let Some((index, msg)) = it_current_items.next() {
                if index <= index_end {
                    continue;
                }
                new_items.push(msg);
            }
            *target = new_items;
        }
        *self.state() = GeneratorState::Completed;
        Ok(())
    }
}
#[derive(Default, Clone, Copy)]
pub(super) enum GeneratorState {
    #[default]
    Empty,
    TagOpened,
    TagClosed,
    Completed,
}

pub(super) struct PosGen<'a> {
    state: GeneratorState,
    args: &'a GeneratorArgs,
    tags_row_found: Vec<Row>,
}
impl<'a> PosGen<'a> {
    pub(super) fn new(args: &'a GeneratorArgs, expected_len: usize) -> Self {
        let mut tags_row_found = Vec::with_capacity(expected_len + 1);
        if args.rows_range.start.0 == 0 {
            tags_row_found.push(args.start.1.clone());
        }
        Self {
            state: Default::default(),
            args,
            tags_row_found,
        }
    }
}

impl<'a> Generator for PosGen<'a> {
    type Item = RowRange;

    fn state(&mut self) -> &mut GeneratorState {
        &mut self.state
    }
    fn next_line_state(&mut self, line_nb: &Row, line: &String) -> crate::Result<GeneratorState> {
        Ok(if is_self_tag_line(&line, TAG_MESSAGE) {
            self.tags_row_found.push(line_nb.clone().into());
            GeneratorState::TagClosed
        } else {
            self.state
        })
    }
    fn finalise(&mut self) -> crate::Result<()> {
        self.tags_row_found.push(self.args.row_end());
        if self.args.is_full_range() && matches!(self.state, GeneratorState::Empty) {
            Err(format!("There's no `<{TAG_MESSAGE} />` in this buffer.").into_error())
        } else {
            Ok(())
        }
    }

    fn args(&mut self) -> &GeneratorArgs {
        self.args
    }

    fn take(&mut self) -> Vec<Self::Item> {
        let len = self.tags_row_found.len();
        let mut ranges = Vec::<RowRange>::with_capacity(len);
        self.tags_row_found.drain(..).reduce(|start, end| {
            ranges.push((start..end).into());
            end
        });
        ranges
    }
}

pub(super) struct MsgGen<'a> {
    state: GeneratorState,
    args: &'a GeneratorArgs,
    messages: Vec<MessageState>,
    nb_messages: usize,
    current_message: MessageState,
    tool_call_generator: Option<ToolCallGen<'a>>,
}

impl<'a> MsgGen<'a> {
    pub(super) fn new(args: &'a GeneratorArgs, expected_len: usize) -> Self {
        Self {
            state: Default::default(),
            args,
            messages: Vec::with_capacity(expected_len),
            nb_messages: 0,
            current_message: MessageState::default(),
            tool_call_generator: None,
        }
    }
}
impl<'a> MsgGen<'a> {
    fn take_tool_calls(&mut self) -> crate::Result<()> {
        let Some(mut tc_gen) = self.tool_call_generator.take() else {
            return Ok(());
        };
        match tc_gen.state {
            GeneratorState::TagOpened => {
                return Err(format!("Tag <{TAG_TOOL_CALL}> not closed.").into_error());
            }
            GeneratorState::TagClosed => {
                let tool_calls = tc_gen.take();
                if !tool_calls.is_empty() {
                    self.current_message.message.tool_calls = Some(tool_calls);
                    // Can't have both tool calls and content, so let's clear it up.
                    self.current_message.message.content.clear();
                }
                Ok(())
            }
            GeneratorState::Completed | GeneratorState::Empty => Ok(()),
        }
    }
}
impl<'a> Generator for MsgGen<'a> {
    type Item = MessageState;

    fn state(&mut self) -> &mut GeneratorState {
        &mut self.state
    }
    fn next_line_state(&mut self, line_nb: &Row, line: &String) -> crate::Result<GeneratorState> {
        let is_message_tag = is_self_tag_line(&line, TAG_MESSAGE);
        if let Some(tc_gen) = self.tool_call_generator.as_mut() {
            crate::log_libuv!(Trace, "TC GEN exist at line {line_nb}");
            if !is_message_tag {
                return tc_gen.next_line(line_nb, line);
            }
            self.take_tool_calls()?;
        }
        if is_message_tag {
            crate::log_libuv!(Trace, "Found tag {TAG_MESSAGE};");
            let mut prev_message = std::mem::take(&mut self.current_message);
            if self.nb_messages > 0 {
                prev_message.message.content = prev_message.message.content.trim_end().to_string();
                self.messages.push(prev_message);
            }
            self.nb_messages += 1;
            parse_tag_line(&line, |key, val, _cols| {
                crate::log_libuv!(Trace, "PARSE : {key} ; {val} ");
                message_setter(key, val, &mut self.current_message)
            });
            Ok(GeneratorState::TagClosed)
        } else if is_open_tag_line(&line, TAG_TOOL_CALL) {
            crate::log_libuv!(Trace, "TOOL_CALL Line found.");
            let mut tc_gen = ToolCallGen::new(self.args);
            tc_gen.next_line(line_nb, line)?;
            self.tool_call_generator = Some(tc_gen);
            Ok(GeneratorState::TagClosed)
        } else {
            self.current_message
                .message
                .content
                .push_str(&format!("{line}\n"));
            Ok(GeneratorState::TagClosed)
        }
    }
    fn finalise(&mut self) -> crate::Result<()> {
        if self.nb_messages > 0 {
            self.take_tool_calls()?;
            let mut prev_message = std::mem::take(&mut self.current_message);
            prev_message.message.content = prev_message.message.content.trim_end().to_string();
            self.messages.push(prev_message);
        }
        let empty = self.nb_messages == 0;
        if self.args.is_full_range() && empty {
            return Err(format!("A Chat file must contain at least one {TAG_MESSAGE} tag.").into_warn());
        }
        Ok(())
    }
    fn args(&mut self) -> &GeneratorArgs {
        self.args
    }

    fn take(&mut self) -> Vec<Self::Item> {
        std::mem::take(&mut self.messages)
    }
}

pub(super) struct ToolCallGen<'a> {
    state: GeneratorState,
    args: &'a GeneratorArgs,
    current: mistral::model::ToolCall,
    tool_calls: Vec<mistral::model::ToolCall>,
    in_json_block: bool,
}
impl<'a> ToolCallGen<'a> {
    pub(super) fn new(args: &'a GeneratorArgs) -> Self {
        Self {
            state: Default::default(),
            args,
            current: Default::default(),
            tool_calls: Default::default(),
            in_json_block: Default::default(),
        }
    }
}
impl<'a> Generator for ToolCallGen<'a> {
    type Item = mistral::model::ToolCall;

    fn state(&mut self) -> &mut GeneratorState {
        &mut self.state
    }
    fn next_line_state(&mut self, _line_nb: &Row, line: &String) -> crate::Result<GeneratorState> {
        if is_open_tag_line(&line, TAG_TOOL_CALL) {
            crate::log_libuv!(Trace, "Found tag {TAG_TOOL_CALL};");
            parse_tag_line(&line, |key, val, _cols| {
                crate::log_libuv!(Trace, "PARSE : {key} ; {val} ");
                tool_call_setter(key, val, &mut self.current)
            });
            return Ok(GeneratorState::TagOpened);
        } else if is_close_tag_line(&line, TAG_TOOL_CALL) {
            self.tool_calls.push(std::mem::take(&mut self.current));
            return Ok(GeneratorState::TagClosed);
        } else if line == "```json" {
            self.in_json_block = true
        } else if line == "```" {
            self.in_json_block = false
        } else if self.in_json_block {
            let args = &mut self.current.function.arguments;
            if !args.is_empty() {
                args.push('\n');
            }
            args.push_str(line);
        }
        Ok(self.state)
    }
    fn finalise(&mut self) -> crate::Result<()> {
        Ok(())
    }
    fn args(&mut self) -> &GeneratorArgs {
        self.args
    }

    fn take(&mut self) -> Vec<Self::Item> {
        std::mem::take(&mut self.tool_calls)
    }
}

pub(super) fn parse_tag_line<Callback>(tag_line: &String, mut parse_arg: Callback)
where
    Callback: FnMut(String, String, model::ColRange),
{
    let mut in_quote = false;
    let mut key = String::new();
    let mut value = String::new();
    let mut column = model::Col(0);
    let mut cols = model::ColRange::default();
    let mut chars = tag_line.chars();
    // Skip Tag
    while let Some(c) = chars.next() {
        column += 1;
        if c == ' ' {
            break;
        }
    }
    let chars = " ".chars().chain(chars);
    for c in chars.collect::<Vec<char>>().windows(2) {
        let c = (c[0], c[1]);
        match c {
            ('>', _) if !in_quote => return,
            ('"', '/') | ('"', ' ') | ('"', '>') if !in_quote => {
                let k = std::mem::take(&mut key);
                let v = std::mem::take(&mut value);
                let c = std::mem::take(&mut cols);
                parse_arg(k, v, c);
            }
            // Detect quotes
            (c, '"') if c != '\\' => {
                if in_quote {
                    cols.end = column;
                } else {
                    cols.start = column + 1;
                }
                in_quote = !in_quote;
            }
            // Fill value
            // ('\\', '"') if in_quote => value.push('"'),
            (_, c) if in_quote => value.push(c),
            // Fill key
            (_, ' ') => (),
            (_, '=') => (),
            (_, c) => key.push(c),
        }
        column += 1;
    }
}

pub(super) fn config_setter(key: String, val: String, chat: &mut ChatState) {
    let metadata = &mut chat.metadata;
    let val = unescape_quote_arg(&val);
    match key.as_str() {
        "name" => metadata.name = val,
        "usage" => metadata.usage = val.into(),
        "description" => metadata.description = val,
        _ => (),
    }
}
pub(super) fn config_getter(key: String, chat: &mut ChatState) -> Option<String> {
    let metadata = &mut chat.metadata;
    Some(escape_quote_arg(match key.as_str() {
        "name" => metadata.name.to_string(),
        "usage" => metadata.usage.to_string(),
        "description" => metadata.description.to_string(),
        _ => return None,
    }))
}

fn message_setter(key: String, val: String, msg: &mut MessageState) {
    let val = unescape_quote_arg(&val);
    match key.as_str() {
        "status" => msg.status.replace_from_str(&val),
        "model" => msg.model.replace_from_str(&val),
        "mode" => msg.mode.replace_from_str(&val),
        "usage" => msg.usage = val.into(),
        // message
        "role" => msg.message.role.replace_from_str(&val),
        "name" => msg.message.name = if val != "" { Some(val) } else { None },
        "tool_call_id" => msg.message.tool_call_id = if val != "" { Some(val) } else { None },
        // params
        "min_tokens" => msg.params.min_tokens = str::parse(&val).ok(),
        "max_tokens" => msg.params.max_tokens = str::parse(&val).ok(),
        _ => (),
    }
}
pub(super) fn message_getter(key: String, msg: &MessageState) -> Option<String> {
    Some(escape_quote_arg(match key.as_str() {
        "status" => msg.status.to_string(),
        "model" => msg.model.to_string(),
        "mode" => msg.mode.to_string(),
        "usage" => msg.usage.to_string(),
        // message
        "role" => msg.message.role.to_string(),
        "name" => option_to_arg(&msg.message.name),
        "tool_call_id" => option_to_arg(&msg.message.tool_call_id),
        // params
        "min_tokens" => option_to_arg(&msg.params.min_tokens),
        "max_tokens" => option_to_arg(&msg.params.max_tokens),
        _ => return None,
    }))
}

fn tool_call_setter(key: String, val: String, tool_call: &mut mistral::model::ToolCall) {
    let val = unescape_quote_arg(&val);
    match key.as_str() {
        "id" => tool_call.id = Some(val),
        "index" => tool_call.index = str::parse(&val).ok(),
        "name" => tool_call.function.name = val,
        "arguments" => tool_call.function.arguments = val,
        _ => (),
    }
}
#[expect(dead_code)]
fn tool_call_getter(key: String, tool_call: &mistral::model::ToolCall) -> Option<String> {
    Some(escape_quote_arg(match key.as_str() {
        "id" => tool_call.id.clone()?,
        "index" => tool_call.index?.to_string(),
        "name" => tool_call.function.name.clone(),
        "arguments" => tool_call.function.arguments.clone(),
        _ => return None,
    }))
}

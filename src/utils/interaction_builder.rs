use serenity::{
    builder::{
        CreateActionRow,
        CreateEmbed,
        CreateInteractionResponse,
        CreateInteractionResponseMessage,
        CreateModal,
        EditInteractionResponse,
    },
    framework::standard::CommandError,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractionBuilderKind {
    Message,
    Modal,
}

pub struct InteractionResponseBuilder {
    custom_id: Option<String>,
    content: Option<String>,
    embed: Option<CreateEmbed>,
    components: Vec<CreateActionRow>,
    kind: InteractionBuilderKind,
    ephemeral: bool,
}

impl InteractionResponseBuilder {
    pub fn new() -> Self {
        Self {
            custom_id: None,
            content: None,
            embed: None,
            components: Vec::new(),
            kind: InteractionBuilderKind::Message,
            ephemeral: false,
        }
    }

    pub fn custom_id(mut self, id: String) -> Self {
        self.custom_id = Some(id);
        self
    }

    pub fn content(mut self, c: String) -> Self {
        self.content = Some(c);
        self
    }

    #[allow(dead_code)]
    pub fn kind(mut self, t: InteractionBuilderKind) -> Self {
        self.kind = t;
        self
    }

    #[allow(dead_code)]
    pub fn embed(mut self, f: impl FnOnce(&mut CreateEmbed) -> &mut CreateEmbed) -> Self {
        let mut embed = CreateEmbed::default();
        f(&mut embed);
        self.embed = Some(embed);
        self
    }

    pub fn components(mut self, rows: Vec<CreateActionRow>) -> Self {
        self.components = rows;
        self
    }

    pub fn make_ephemeral(mut self) -> Self {
        self.ephemeral = true;
        self
    }

    pub fn into_create_response(self) -> Result<CreateInteractionResponse, CommandError> {
        if self.kind == InteractionBuilderKind::Modal {
            let id = if let Some(id) = self.custom_id {
                Ok(id)
            } else {
                Err(CommandError::from(
                    "A custom id is required for a modal",
                ))
            }?;
            let title = if let Some(content) = self.content {
                Ok(content)
            } else {
                Err(CommandError::from(
                    "content is required in a modal",
                ))
            }?;
            return Ok(CreateInteractionResponse::Modal(
                CreateModal::new(id, title).components(self.components),
            ));
        }

        let mut resp = CreateInteractionResponseMessage::new();
        if let Some(embed) = self.embed {
            resp = resp.embed(embed);
        }

        if let Some(content) = self.content {
            resp = resp.content(content);
        }
        if !self
            .components
            .is_empty()
        {
            resp = resp.components(self.components);
        }
        resp = resp.ephemeral(self.ephemeral);

        Ok(CreateInteractionResponse::Message(resp))
    }

    pub fn into_edit_response(self) -> EditInteractionResponse {
        let mut resp = EditInteractionResponse::new();

        if let Some(embed) = self.embed {
            resp = resp.embed(embed);
        }
        if let Some(content) = self.content {
            resp = resp.content(content);
        }
        if !self
            .components
            .is_empty()
        {
            resp = resp.components(self.components);
        }

        resp
    }
}

impl TryFrom<InteractionResponseBuilder> for CreateInteractionResponse {
    type Error = CommandError;

    fn try_from(other: InteractionResponseBuilder) -> Result<Self, Self::Error> {
        other.into_create_response()
    }
}

impl From<InteractionResponseBuilder> for EditInteractionResponse {
    fn from(other: InteractionResponseBuilder) -> EditInteractionResponse {
        other.into_edit_response()
    }
}

impl Default for InteractionResponseBuilder {
    fn default() -> Self {
        Self::new()
    }
}

use serenity::{
    builder::{
        CreateComponents,
        CreateEmbed,
        CreateInteractionResponse,
        EditInteractionResponse,
    },
    model::application::interaction::InteractionResponseType,
};

pub struct InteractionResponseBuilder {
    custom_id: Option<String>,
    content: Option<String>,
    embed: Option<CreateEmbed>,
    components: Option<CreateComponents>,
    kind: InteractionResponseType,
    ephemeral: bool,
}

impl InteractionResponseBuilder {
    pub fn new() -> Self {
        Self {
            custom_id: None,
            content: None,
            embed: None,
            components: None,
            kind: InteractionResponseType::ChannelMessageWithSource,
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
    pub fn kind(mut self, t: InteractionResponseType) -> Self {
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

    pub fn components(
        mut self,
        f: impl FnOnce(&mut CreateComponents) -> &mut CreateComponents,
    ) -> Self {
        let mut comps = CreateComponents::default();
        f(&mut comps);
        self.components = Some(comps);
        self
    }

    pub fn make_ephemeral(mut self) -> Self {
        self.ephemeral = true;
        self
    }

    pub fn into_create_response(self) -> CreateInteractionResponse<'static> {
        let mut resp = CreateInteractionResponse::default();

        resp.kind(self.kind)
            .interaction_response_data(|d| {
                if let Some(embed) = self.embed {
                    d.set_embed(embed);
                }

                if self.kind == InteractionResponseType::Modal {
                    if let Some(content) = self.content {
                        d.title(content);
                    }
                } else if let Some(content) = self.content {
                    d.content(content);
                }

                if let Some(components) = self.components {
                    d.components(|c| {
                        *c = components;
                        c
                    });
                }
                if let Some(id) = self.custom_id {
                    d.custom_id(id);
                }
                d.ephemeral(self.ephemeral)
            });

        resp
    }

    pub fn into_edit_response(self) -> EditInteractionResponse {
        let mut resp = EditInteractionResponse::default();

        if let Some(embed) = self.embed {
            resp.set_embed(embed);
        }
        if let Some(content) = self.content {
            resp.content(content);
        }
        if let Some(components) = self.components {
            resp.components(|c| {
                *c = components;
                c
            });
        }

        resp
    }
}

impl From<InteractionResponseBuilder> for CreateInteractionResponse<'_> {
    fn from(other: InteractionResponseBuilder) -> CreateInteractionResponse<'static> {
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

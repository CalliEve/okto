use serenity::{http::{Http, request::RequestBuilder, routing::RouteInfo}, Result, model::{guild::{Role, GuildInfo, PartialGuild}, Permissions}};


pub async fn get_all_guilds(http: &Http) -> Result<Vec<PartialGuild>> {
    let mut res: Vec<PartialGuild> = Vec::new();

    loop {
        let guilds: Vec<GuildInfo> = http.fire(RequestBuilder::new(RouteInfo::GetGuilds {
                before: None,
                after: res.last().map(|g| g.id.0),
                limit: 200
            }).build()).await?;
        
        let mut p_guilds: Vec<PartialGuild> = Vec::with_capacity(guilds.len());
        for g in guilds {
            let p_g = g.id.to_partial_guild(http).await?;
            p_guilds.push(p_g);
        }

        if p_guilds.len() < 200 {
            res.append(&mut p_guilds);
            break;
        }
        res.append(&mut p_guilds);
    }

    Ok(res)
}

pub fn get_roles_with_permission(guild: &PartialGuild, permissions: Permissions) -> Vec<Role> {
    let mut roles = guild.roles.iter().map(|(_,v)| v).filter(|r| r.has_permission(permissions)).cloned().collect::<Vec<Role>>();
    roles.sort();
    roles.reverse();
    roles.into_iter().take(9).collect()
}


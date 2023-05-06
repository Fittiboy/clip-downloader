use worker::{event, Cache, Context, Env, Request, Response, Result};

mod twitch;
use twitch::{Client, ClipId};

#[event(fetch)]
async fn main(req: Request, env: Env, ctx: Context) -> Result<Response> {
    let cache = Cache::default();
    if let Some(response) = cache.get(&req, false).await? {
        Ok(response)
    } else {
        redirect_to_clip(req, &env, &ctx, cache).await
    }
}

async fn redirect_to_clip(r: Request, e: &Env, c: &Context, cache: Cache) -> Result<Response> {
    let clip_id = match id_from(&r) {
        Some(id) if !id.is_empty() => id,
        _ => return missing_clip_id(),
    };
    let client = Client::authenticated(e).await?;
    let clip = client.fetch_clip(clip_id).await?;
    let url = clip.media_url()?;
    let response = Response::redirect(url.clone())?;
    c.wait_until(async move {
        cache.put(&r, response).await.ok();
    });
    Response::redirect(url)
}

fn id_from(req: &Request) -> Option<ClipId> {
    match domain_from(req)?.as_str() {
        "clips" => Some(clips_id(req)),
        "www" => www_id(req),
        _ => None,
    }
}

fn domain_from(req: &Request) -> Option<String> {
    Some(req.url().ok()?.domain()?.split_once('.')?.0.to_string())
}

fn clips_id(req: &Request) -> ClipId {
    const LEADING_SLASH: usize = 1;
    req.path().split_off(LEADING_SLASH)
}

fn www_id(req: &Request) -> Option<ClipId> {
    Some(req.url().ok()?.path_segments()?.last()?.to_string())
}

fn missing_clip_id() -> Result<Response> {
    Response::error(
        concat!(
            "Hey! This site is used for my clip downloading tool!\n\n",
            "It appears that this is not a Twitch clip URL!\n",
            "Make sure that the URL you're starting with is either this:\n\t",
            "https://www.twitch.tv/.../clip/...\n\nturned into:\n\t",
            "https://www.fitti.io/.../clip/...\n\nor this:\n\t",
            "https://clips.twitch.tv/...\n\nturned into:\n\t",
            "https://clips.fitti.io/...\n\n\n\t\t",
            "If you think this is a mistake, email me at dev @ this domain!"
        ),
        404,
    )
}

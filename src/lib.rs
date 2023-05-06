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

fn id_from(req: &Request) -> ClipId {
    const LEADING_SLASH: usize = 1;
    req.path().split_off(LEADING_SLASH)
}

async fn redirect_to_clip(r: Request, e: &Env, c: &Context, cache: Cache) -> Result<Response> {
    let clip_id = id_from(&r);
    let client = Client::authenticated(e).await?;
    let clip = client.fetch_clip(clip_id).await?;
    let url = clip.media_url()?;
    let response = Response::redirect(url.clone())?;
    c.wait_until(async move {
        cache.put(&r, response).await.ok();
    });
    Response::redirect(url)
}

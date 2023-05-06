use serde::Deserialize;
use worker::{
    event, Context, Env, Fetch, Headers, Method::Get, Method::Post, Request, RequestInit, Response,
    Result, Url,
};

#[event(fetch)]
async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let clip_id = clip_id(&req);
    let token = token(&env).await?;
    let clip = clip(&env, token, clip_id).await?;
    let url = clip.media_url()?;
    Response::redirect(url)
}

fn clip_id(req: &Request) -> String {
    let mut path = req.path();
    path.split_off(1)
}

async fn token(env: &Env) -> Result<String> {
    let client_id = env.secret("TWITCH_CLIENT_ID")?;
    let client_secret = env.secret("TWITCH_CLIENT_SECRET")?;
    let mut init = RequestInit::new();
    let body = format!(
        "client_id={}&client_secret={}&grant_type=client_credentials",
        client_id.to_string(),
        client_secret.to_string(),
    );
    let mut headers = Headers::new();
    headers.set("Content-Type", "application/x-www-form-urlencoded")?;
    let init = init
        .with_method(Post)
        .with_body(Some(body.into()))
        .with_headers(headers);
    let auth_request = Request::new_with_init("https://id.twitch.tv/oauth2/token", init)?;
    let client = Fetch::Request(auth_request);
    let mut response = client.send().await?;
    let response: AuthResponse = response.json().await?;
    Ok(response.access_token)
}

#[derive(Debug, Deserialize)]
struct AuthResponse {
    access_token: String,
}

async fn clip(env: &Env, token: String, id: String) -> Result<Clip> {
    let request_url = format!("https://api.twitch.tv/helix/clips?id={}", id);
    let mut clips_request = Request::new(&request_url, Get)?;
    let headers = clips_request.headers_mut()?;
    let client_id = env.secret("TWITCH_CLIENT_ID")?;
    headers.set("Client-Id", &client_id.to_string())?;
    headers.set("Authorization", &format!("Bearer {}", token))?;
    let client = Fetch::Request(clips_request);
    let response: Clips = client.send().await?.json().await?;
    let Clips { data: [clip] } = response;
    Ok(clip)
}

#[derive(Debug, Deserialize)]
struct Clip {
    thumbnail_url: String,
}

impl Clip {
    fn media_url(&self) -> Result<Url> {
        let url: &str = &self.thumbnail_url;
        let url = format!("{}.mp4", url.split_once("-preview").unwrap().0);
        Ok(Url::parse(&url)?)
    }
}

#[derive(Debug, Deserialize)]
struct Clips {
    data: [Clip; 1],
}

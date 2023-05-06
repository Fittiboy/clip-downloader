use serde::Deserialize;
use worker::{
    event, Context, Env, Fetch, Headers, Method::Get, Method::Post, Request, RequestInit, Response,
    Result, Url,
};

#[event(fetch)]
async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let clip_id = id_from_path(&req);
    let client = TwitchClient::authenticated(&env).await?;
    let clip = client.fetch_clip(clip_id).await?;
    let url = clip.media_url()?;
    Response::redirect(url)
}

fn id_from_path(req: &Request) -> ClipId {
    const LEADING_SLASH: usize = 1;
    req.path().split_off(LEADING_SLASH)
}

type ClipId = String;

#[derive(Debug)]
struct TwitchClient {
    client_id: String,
    access_token: String,
}

impl TwitchClient {
    async fn authenticated(env: &Env) -> Result<Self> {
        let setup = TwitchClientSetup::new(env)?;
        let mut auth_response = setup.auth_request()?.send().await?;
        let AuthResponse { access_token } = auth_response.json().await?;
        Ok(Self {
            client_id: setup.client_id,
            access_token,
        })
    }

    async fn fetch_clip(&self, id: ClipId) -> Result<Clip> {
        let url = format!("https://api.twitch.tv/helix/clips?id={}", id);
        let response = self.fetch_single_clip(url).await?;
        let Clips { data: [clip] } = response;
        Ok(clip)
    }

    async fn fetch_single_clip(&self, url: String) -> Result<Clips> {
        let mut clips_request = Request::new(&url, Get)?;
        self.set_auth_headers(&mut clips_request)?;
        let client = Fetch::Request(clips_request);
        client.send().await?.json().await
    }

    fn set_auth_headers(&self, req: &mut Request) -> Result<()> {
        let headers = req.headers_mut()?;
        headers.set("Client-Id", &self.client_id)?;
        headers.set("Authorization", &format!("Bearer {}", self.access_token))?;
        Ok(())
    }
}

#[derive(Debug)]
struct TwitchClientSetup {
    client_id: ClientId,
    client_secret: ClientSecret,
}

type ClientId = String;
type ClientSecret = String;

impl TwitchClientSetup {
    fn new(env: &Env) -> Result<Self> {
        let client_id = env.secret("TWITCH_CLIENT_ID")?.to_string();
        let client_secret = env.secret("TWITCH_CLIENT_SECRET")?.to_string();
        Ok(Self {
            client_id,
            client_secret,
        })
    }

    fn auth_request(&self) -> Result<Fetch> {
        let body = self.auth_body();
        let headers = Self::auth_headers()?;
        let init = Self::auth_init(body, headers);
        let auth_request = Request::new_with_init("https://id.twitch.tv/oauth2/token", &init)?;
        Ok(Fetch::Request(auth_request))
    }

    fn auth_body(&self) -> String {
        format!(
            "client_id={}&client_secret={}&grant_type=client_credentials",
            self.client_id, self.client_secret,
        )
    }

    fn auth_headers() -> Result<Headers> {
        let mut headers = Headers::new();
        headers.set("Content-Type", "application/x-www-form-urlencoded")?;
        Ok(headers)
    }

    fn auth_init(body: String, headers: Headers) -> RequestInit {
        let mut init = RequestInit::new();
        init.with_method(Post)
            .with_body(Some(body.into()))
            .with_headers(headers);
        init
    }
}

#[derive(Debug, Deserialize)]
struct AuthResponse {
    access_token: String,
}

#[derive(Debug, Deserialize)]
struct Clip {
    thumbnail_url: String,
}

impl Clip {
    fn media_url(&self) -> Result<Url> {
        // The Twitch API returns a a clip object that contains not a link to the file, but to the
        // thumbnail of the clip. By splitting off the part that starts with "-preview", and
        // replacing it with ".mp4," we get the direct link to the clip's media file.
        let url: &str = &self.thumbnail_url;
        let url = format!("{}.mp4", url.split_once("-preview ").unwrap().0);
        Ok(Url::parse(&url)?)
    }
}

#[derive(Debug, Deserialize)]
struct Clips {
    data: [Clip; 1],
}

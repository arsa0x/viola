use mlua::{Error as LuaError, UserData, UserDataMethods};

#[derive(Clone)]
pub struct LuaHttpResponse {
    pub status: u16,
    pub body: String,
}

impl UserData for LuaHttpResponse {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("status", |_, this, ()| Ok(this.status));

        methods.add_method("body", |_, this, ()| Ok(this.body.clone()));
    }
}

#[derive(Clone)]
pub struct LuaHttpClient {
    pub client: reqwest::Client,
}

impl UserData for LuaHttpClient {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_async_method("get", |_, this, url: String| {
            let client = this.client.clone();
            async move {
                let res = client.get(&url).send().await.map_err(LuaError::external)?;

                let status = res.status().as_u16();
                let body = res.text().await.map_err(LuaError::external)?;

                Ok(LuaHttpResponse { status, body })
            }
        });

        methods.add_async_method("post", |_, this, (url, body_data): (String, String)| {
            let client = this.client.clone();
            async move {
                let res = client
                    .post(&url)
                    .header("content-type", "application/json")
                    .body(body_data)
                    .send()
                    .await
                    .map_err(LuaError::external)?;

                let status = res.status().as_u16();
                let body = res.text().await.map_err(LuaError::external)?;

                Ok(LuaHttpResponse { status, body })
            }
        });
    }
}

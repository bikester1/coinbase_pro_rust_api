use std::fmt::Debug;

use hmac::{
    Hmac,
    Mac,
};
use log::debug;
use reqwest::Method;
use serde::de::DeserializeOwned;
use serde::Serialize;
use sha2::Sha256;

#[cfg(not(feature = "mock"))]
use mocked::*;
#[cfg(feature = "mock")]
use mocks::*;

use crate::api::APIKeyData;
use crate::errors::Error::{
    CBProServerErrorVariant,
    RequestBuilderCloningError,
    SerdeError,
    SerdeSerializationError,
};
use crate::errors::{
    CBProServerError,
    Error,
};

#[cfg(feature = "mock")]
mod mocks {
    pub use crate::mocked::{
        MockClient as Client,
        MockRequestBuilder as RequestBuilder,
    };
}

mod mocked {
    pub use reqwest::{
        Client,
        RequestBuilder,
    };
}

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone)]
pub enum RequestMethod {
    GET,
    POST,
}

impl Into<String> for RequestMethod {
    fn into(self) -> String {
        match self {
            RequestMethod::GET => "GET".to_string(),
            RequestMethod::POST => "POST".to_string(),
        }
    }
}

impl Into<reqwest::Method> for RequestMethod {
    fn into(self) -> Method {
        match self {
            RequestMethod::GET => reqwest::Method::GET,
            RequestMethod::POST => reqwest::Method::POST,
        }
    }
}

/// # CBRequestBuilder
///
/// This is the low level api for creating your own request to coinbase.
/// It allows for several features including signed requests, pagenated responses, custom user agent, and
/// custom url in case you would like to query an alternate endpoint or coinbase changes urls.
///
///
pub struct CBRequestBuilder {
    client: Client,
    user_agent: String,
    credentials: Option<APIKeyData>,
    end_point: String,
    query_params: Vec<(String, String)>,
    body: String,
    url: String,
    method: RequestMethod,
}

impl CBRequestBuilder {
    pub fn new(client: &Client, user_agent: String) -> Self {
        CBRequestBuilder {
            client: client.clone(),
            user_agent,
            credentials: None,
            end_point: "".to_string(),
            query_params: vec![],
            body: "".to_string(),
            url: "https://api.exchange.coinbase.com".to_string(),
            method: RequestMethod::GET,
        }
    }

    pub fn set_url(mut self, url: String) -> Self {
        self.url = url;
        self
    }

    pub fn add_query_param(mut self, key: String, value: String) -> Self {
        self.query_params.push((key, value));
        self
    }

    pub fn try_add_query_param(self, key: String, value: Option<String>) -> Self {
        if let Some(val) = value {
            self.add_query_param(key, val)
        } else {
            self
        }
    }

    pub fn set_endpoint(mut self, end_point: String) -> Self {
        self.end_point = end_point;
        self
    }

    pub fn sign(mut self, credentials: APIKeyData) -> Self {
        self.credentials = Some(credentials);
        self
    }

    pub fn set_method(mut self, method: RequestMethod) -> Self {
        self.method = method;
        self
    }

    pub fn set_body(mut self, body: impl Serialize) -> Result<Self, Error> {
        self.body = serde_json::to_string(&body).map_err(|err| SerdeSerializationError(err))?;
        Ok(self)
    }

    pub async fn exec_pagenated<O>(self) -> Result<Vec<O>, Error>
    where
        O: DeserializeOwned + Debug,
    {
        let url_string = format!("{}{}", self.url, self.end_point);

        let request = self
            .client
            .request(self.method.clone().into(), url_string)
            .header("User-Agent", self.user_agent.clone())
            .header("Content-Type", "application/json");

        let page_len = 1000;

        let mut after: Option<String> = None;
        let mut ret_vec = Vec::new();

        // Loop until we don't get the max number of items
        loop {
            let mut request_clone = request.try_clone().ok_or(RequestBuilderCloningError)?;

            let mut params = self.query_params.clone();

            if let Some(aft) = after {
                let mut aft_param = vec![("after".to_string(), aft)];
                params.append(&mut aft_param);
            }

            request_clone = request_clone.query(&params);

            if let Some(creds) = self.credentials.clone() {
                request_clone = request_clone.sign_request(
                    creds,
                    self.end_point.clone(),
                    Some(params),
                    self.method.clone().into(),
                    self.body.clone(),
                );
            }

            let resp = request_clone
                .try_clone()
                .ok_or(RequestBuilderCloningError)?
                .send()
                .await?;

            after = resp
                .headers()
                .get("cb-after")
                .map(|val| val.clone().to_str().unwrap().to_string());

            let body = resp.text().await?;
            let serde_result: Result<Vec<O>, serde_json::Error> =
                serde_json::from_str(body.as_str());
            let mut parsed_response: Vec<O> = match serde_result {
                Ok(target) => target,
                Err(error) => {
                    let serde_cberror_result: Result<CBProServerError, serde_json::Error> =
                        serde_json::from_str(body.as_str());

                    return if let Ok(return_struct) = serde_cberror_result {
                        Err(CBProServerErrorVariant(return_struct))
                    } else {
                        Err(SerdeError {
                            source: error,
                            string: body,
                        })
                    };
                }
            };
            let len = parsed_response.len();

            ret_vec.append(&mut parsed_response);

            if len != page_len {
                break;
            }
        }

        Ok(ret_vec)
    }

    pub async fn exec<O>(self) -> Result<O, Error>
    where
        O: DeserializeOwned + Debug,
    {
        let url_string = format!("{}{}", self.url, self.end_point);

        let mut request = self
            .client
            .request(self.method.clone().into(), url_string)
            .header("User-Agent", self.user_agent.clone())
            .body(reqwest::Body::from(self.body.clone()))
            .header("Content-Type", "application/json")
            .query(&self.query_params);

        if let Some(creds) = self.credentials.clone() {
            request = request.sign_request(
                creds,
                self.end_point.clone(),
                Some(self.query_params.clone()),
                self.method.clone().into(),
                self.body.clone(),
            );
        }

        let req = request.try_clone().unwrap().build().unwrap();

        let response_body = request.send().await?.text().await?;

        let serde_result: Result<O, serde_json::Error> =
            serde_json::from_str(response_body.as_str());

        // If parsing of target type fails then fall back on a standard CBPro Error Message.
        // If this is not a standard CBPro Error Message then return the parsing error for the
        // target type.
        // This approach was chosen over the past untagged enum because it provides better
        // error messages
        return match serde_result {
            Ok(target) => Ok(target),
            Err(error) => {
                let serde_cberror_result: Result<CBProServerError, serde_json::Error> =
                    serde_json::from_str(response_body.as_str());

                if let Ok(return_struct) = serde_cberror_result {
                    Err(CBProServerErrorVariant(return_struct))
                } else {
                    Err(SerdeError {
                        source: error,
                        string: response_body,
                    })
                }
            }
        };
    }
}

pub(crate) trait SignRequest {
    fn sign_request(
        self,
        credentials: APIKeyData,
        path: String,
        query_params: Option<Vec<(String, String)>>,
        method: String,
        body: String,
    ) -> Self;
}

impl SignRequest for RequestBuilder {
    fn sign_request(
        mut self,
        credentials: APIKeyData,
        mut path: String,
        query_params: Option<Vec<(String, String)>>,
        method: String,
        body: String,
    ) -> Self {
        let timestamp = chrono::Utc::now().timestamp();

        // Append query params to the end of the path
        if query_params.is_some() {
            path = format!("{}?", path);
            for query in query_params.unwrap().iter() {
                path = format!("{}{}={}&", path, query.0, query.1);
            }
            // Remove last character so that queries do not end with an &
            path.pop();
        }

        // Format message to be signed
        let message = format!("{}{}{}{}", timestamp, method, path, body);

        // Sign message
        if let Ok(mut mac) =
            HmacSha256::new_from_slice(base64::decode(credentials.secret).unwrap().as_slice())
        {
            mac.update(message.as_bytes());
            let signature = mac.finalize().into_bytes();

            // Build request
            self = self
                .header("CB-ACCESS-KEY", credentials.key)
                .header("CB-ACCESS-SIGN", base64::encode(signature))
                .header("CB-ACCESS-TIMESTAMP", timestamp)
                .header("CB-ACCESS-PASSPHRASE", credentials.passphrase);
        }

        self
    }
}

// This function takes a given reqwest client and user_agent string and asyncronously returns
// a result or an error.
// Optionally this function takes a product_id string in which case this function will return only
// the single product specified.
// If an invalid product is specified you will not get an error but a valid response with a 404
// status code. To catch errors it is recommended that the status code of the response is checked.
// pub(crate) mod raw {
//     use std::collections::HashMap;
//     use crate::data_structures::{APIKeyData, Level};
//     use crate::errors::Error;
//     use crate::errors::Error::ReqwestError;
//     use crate::requests::HmacSha256;
//     use hmac::Mac;
//     use reqwest::header::HeaderMap;
//
//     pub async fn get_all_products_raw(client: &reqwest::Client, user_agent: String, product_id: Option<String>) -> Result<reqwest::Response, Error> {
//         let suffix = match product_id {
//             None => { "".to_string() }
//             Some(id) => { format!("/{}", id) }
//         };
//
//         client.request(reqwest::Method::GET, format!("https://api.exchange.coinbase.com/products{}", suffix))
//             .header("User-Agent", user_agent)
//             .send()
//             .await
//             .map_err(|err| ReqwestError(err))
//     }
//
//     pub async fn get_product_book_raw(client: &reqwest::Client, user_agent: String, product_id: String, level: Option<Level>) -> Result<reqwest::Response, Error> {
//         let level = level.unwrap_or(Level::One);
//
//         client.request(reqwest::Method::GET, format!("https://api.exchange.coinbase.com/products/{}/book", product_id))
//             .header("User-Agent", user_agent.clone())
//             .query(&[("level", level.clone().as_string())])
//             .send()
//             .await
//             .map_err(|err| ReqwestError(err))
//     }
//
//     #[inline]
//     pub(crate) async fn signed_request(
//         client: &reqwest::Client,
//         user_agent: String,
//         credentials: APIKeyData,
//         mut path: String,
//         query_params: Option<Vec<(String, String)>>
//     ) -> Result<reqwest::Response, Error> {
//         let timestamp = chrono::Utc::now().timestamp();
//         let body = "";
//         let method = "GET";
//
//         // Append query params to the end of the path
//         if query_params.is_some() {
//             path = format!("{}?", path);
//             for query in query_params.unwrap().iter() {
//                 path = format!("{}{}={}&", path, query.0, query.1);
//             }
//             // Remove last character so that queries do not end with an &
//             path.pop();
//         }
//
//         // Format message to be signed
//         let message = format!("{}{}{}{}", timestamp, method, path, body);
//
//         // Sign message
//         let mut mac = HmacSha256::new_from_slice(base64::decode(credentials.secret).unwrap().as_slice()).map_err(|err| Error::Custom(format!("Invalid secret length: {}", err)))?;
//         mac.update(message.as_bytes());
//         let signature = mac.finalize().into_bytes();
//
//         // Build request
//         let mut req = client.request(reqwest::Method::GET, format!("https://api.exchange.coinbase.com{}", path))
//             .header("User-Agent", user_agent.clone())
//             .header("CB-ACCESS-KEY", credentials.key)
//             .header("CB-ACCESS-SIGN", base64::encode(signature))
//             .header("CB-ACCESS-TIMESTAMP", timestamp)
//             .header("CB-ACCESS-PASSPHRASE", credentials.passphrase);
//
//         req.send()
//             .await
//             .map_err(|err| ReqwestError(err))
//     }
//
//     pub(crate) async fn get_all_accounts_raw(client: &reqwest::Client, user_agent: String, credentials: APIKeyData) -> Result<reqwest::Response, Error> {
//         let path = format!("/accounts");
//         signed_request(client, user_agent, credentials, path, None).await
//     }
//
//     pub(crate) async fn get_single_account_raw(client: &reqwest::Client, user_agent: String, credentials: APIKeyData, account_id: String) -> Result<reqwest::Response, Error> {
//         let path = format!("/accounts/{}", account_id);
//         signed_request(client, user_agent, credentials, path, None).await
//     }
//
//     pub(crate) async fn get_single_account_holds_raw(client: &reqwest::Client, user_agent: String, credentials: APIKeyData, account_id: String) -> Result<reqwest::Response, Error> {
//         let path = format!("/accounts/{}/holds", account_id);
//         signed_request(client, user_agent, credentials, path, None).await
//     }
//
//     pub(crate) async fn get_single_account_ledger_raw(client: &reqwest::Client, user_agent: String, credentials: APIKeyData, account_id: String, headers: Option<HeaderMap>, query_params: Option<Vec<(String, String)>>) -> Result<reqwest::Response, Error> {
//         let path = format!("/accounts/{}/ledger", account_id);
//         signed_request(client, user_agent, credentials, path, query_params).await
//     }
//
//     pub(crate) async fn get_single_account_transfers_raw(client: &reqwest::Client, user_agent: String, credentials: APIKeyData, account_id: String) -> Result<reqwest::Response, Error> {
//         let path = format!("/accounts/{}/transfers", account_id);
//         signed_request(client, user_agent, credentials, path, None).await
//     }
//
//     pub(crate) async fn get_fees_raw(client: &reqwest::Client, user_agent: String, credentials: APIKeyData) -> Result<reqwest::Response, Error> {
//         let path = format!("/fees");
//         signed_request(client, user_agent, credentials, path, None).await
//     }
// }

// pub(crate) async fn get_paginated<T, P, F>(
//     request: fn(P, page_len: u64, after: Option<String>) -> F,
//     params: P,
//     page_len: Option<u64>,
// ) -> Result<Vec<T>, Error>
// where
//     T: DeserializeOwned,
//     P: Clone,
//     F: Future<Output = Result<reqwest::Response, Error>>,
// {
//     let page_len = page_len.unwrap_or(1000);
//
//     let mut after: Option<String> = None;
//     let mut ret_vec = Vec::new();
//
//     // Loop until we don't get the max number of items
//     loop {
//         let resp = request(params.clone(), page_len, after).await?;
//         after = Some(
//             resp.headers()
//                 .get("cb-after")
//                 .unwrap()
//                 .clone()
//                 .to_str()
//                 .unwrap()
//                 .to_string(),
//         );
//         let mut parsed_response = resp
//             .json::<CBProResponse<Vec<T>>>()
//             .await
//             .map_err(|err| SerdeJSONParseError { message: "".to_string(), source: err })?
//             .map_err(|err| Error::Custom(format!("{:?}", err)))?;
//         let len = parsed_response.len();
//         ret_vec.append(&mut parsed_response);
//
//         if len != page_len as usize {
//             break;
//         }
//     }
//
//     Ok(ret_vec)
// }

// pub(crate) async fn get_single_account_ledger(client: &Client, user_agent: String, credentials: APIKeyData, account_id: String) -> Result<Vec<crate::data_structures::Ledger>, Error> {
//     let mut after: Option<HeaderValue> = None;
//     let mut ret_vec = Vec::new();
//
//     // Loop until we don't get the max number of items
//     loop {
//
//
//         let query_params = if after.is_some() {
//             let aft_str = after.unwrap();
//             let aft_str = aft_str.to_str().map_err(|err| Error::Custom(format!("{}", err)))?.to_string();
//             Some([("after".to_string(), aft_str)].to_vec())
//         } else {
//             None
//         };
//
//         //Some([("after", after.to_str().map_err(|x| Error::Custom(format!("")))?)][..]) }
//
//         let resp = raw::get_single_account_ledger_raw(client, user_agent.clone(), credentials.clone(), account_id.clone(), None, query_params).await?;
//         after = Some(resp.headers().get("cb-after").unwrap().clone());
//         let mut parsed_response = resp.json::<CBProResponse<Vec<crate::data_structures::Ledger>>>()
//             .await
//             .map_err(|err| Error::Custom(format!("{}", err)))?
//             .map_err(|err| Error::Custom(format!("{:?}", err)))?;
//         let len = parsed_response.len();
//         ret_vec.append(&mut parsed_response);
//
//         if len != 1000 {break;}
//     }
//
//     println!("{}", ret_vec.len());
//     Ok(ret_vec)
// }
//
// pub(crate) async fn get_single_account_holds(client: &Client, user_agent: String, credentials: APIKeyData, account_id: String) -> Result<Vec<crate::data_structures::Hold>, Error> {
//     let resp = raw::get_single_account_holds_raw(client, user_agent, credentials, account_id).await?;
//     let after = resp.headers().get("cb-after").unwrap().clone();
//
//
//
//     resp.json::<CBProResponse<Vec<crate::data_structures::Hold>>>()
//         .await
//         .map_err(|err| Error::Custom(format!("{}", err)))?
//         .map_err(|err| Error::Custom(format!("{:?}", err)))
// }
//
// pub(crate) async fn get_single_account(client: &Client, user_agent: String, credentials: APIKeyData, account_id: String) -> Result<crate::data_structures::Account, Error> {
//     raw::get_single_account_raw(client, user_agent, credentials, account_id)
//         .await?
//         .json::<CBProResponse<crate::data_structures::Account>>()
//         .await
//         .map_err(|err| Error::Custom(format!("{}", err)))?
//         .map_err(|err| Error::Custom(format!("{:?}", err)))
// }
//
// pub(crate) async fn get_fees(client: &Client, user_agent: String, credentials: APIKeyData) -> Result<Fees, Error> {
//     raw::get_fees_raw(client, user_agent, credentials)
//         .await?
//         .json::<CBProResponse<Fees>>()
//         .await
//         .map_err(|err| Error::Custom(format!("{}", err)))?
//         .map_err(|err| Error::Custom(format!("{:?}", err)))
// }
//
// pub(crate) async fn get_product_book(client: Client, user_agent: String, product_id: String, level: Option<Level>) -> Result<ProductBook, Error> {
//     raw::get_product_book_raw(&client, user_agent, product_id, level)
//         .await?
//         .json::<CBProResponse<ProductBook>>()
//         .await
//         .map_err(|err| ReqwestError(err))?
//         .map_err(|err| CBProError(err))
// }
//
// /// This function is similar to [get_all_products_raw] but will attempt to format the response into
// /// a valid Vec of Product information. It by default does not take a product ID and instead returns
// /// all products. See [get_product] for retrieving a single product.
// pub(crate) async fn get_all_products(client: &Client, user_agent: String) -> Result<Vec<Product>, Error> {
//     raw::get_all_products_raw(client, user_agent, None)
//         .await?
//         .json::<Vec<Product>>()
//         .await
//         .map_err(|err| ReqwestError(err))
// }
//
// /// This function trys to call and parse a [get_all_products_raw] call into a single product.
// /// If parsing fails you will get a reqwest decoding error.
// /// For multiple products use [get_all_products].
// pub(crate) async fn get_product(client: &Client, user_agent: String, product_id: String) -> Result<Product, Error> {
//     raw::get_all_products_raw(client, user_agent, Some(product_id))
//         .await?
//         .json::<Product>()
//         .await
//         .map_err(|err| ReqwestError(err))
// }

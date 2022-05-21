mod tests1 {
    use tokio;

    use crate::api::{
        APIKeyData,
        CBProAPI,
        Level,
        SubscriptionBuilder,
    };
    use crate::datastructs::products::{
        Orders,
        ProductBook,
    };
    use crate::keys::{
        KEY,
        SECRET,
    };

    #[test]
    fn cb_product_book_level_1_bid_count() {
        let client = CBProAPI::default();
        let response = client.get_product_book("ETH-USD".to_string(), Some(Level::One));
        let blocked_response = tokio_test::block_on(response);
        let unwrapped = blocked_response.unwrap();

        assert_eq!(1, unwrapped.asks.len());
        assert_eq!(1, unwrapped.bids.len());
    }

    #[test]
    fn cb_product_book_level_2_bid_count() {
        let client = CBProAPI::default();
        let response = client.get_product_book("ETH-USD".to_string(), Some(Level::Two));
        let blocked_response = tokio_test::block_on(response);
        let unwrapped = blocked_response.unwrap();

        assert!(unwrapped.asks.len() > 1);
        assert!(unwrapped.bids.len() > 1);
    }

    #[test]
    fn cb_404_invalid_product_parsed() {
        let client = CBProAPI::default();
        let response = client.get_product("Non_existant_product".to_string());
        let blocked_response = tokio_test::block_on(response);

        assert!(
            blocked_response.is_err(),
            "Expected a decoding error got valid response"
        );
    }

    #[test]
    fn cb_eth_product_parsed() {
        let client = CBProAPI::default();
        let response = client.get_product("ETH-USD".to_string());
        let blocked_response = tokio_test::block_on(response);
        let valid_resp = blocked_response.unwrap();
        assert_eq!("ETH-USD", valid_resp.id);
        assert_eq!(0.01, valid_resp.quote_increment);
        assert!(valid_resp.max_slippage_percentage.is_some());
    }

    // todo!() possibly not needed due to parsing changes
    #[test]
    fn test_raw_conversion() {
        let raw_order = Orders {
            price: 100.0,
            size: 10.0,
            num_orders: 24,
        };

        let raw_product_book = ProductBook {
            bids: vec![raw_order.clone()],
            asks: vec![raw_order.clone()],
            sequence: 1000.0,
            auction_mode: Some(false),
            auction: None,
        };

        let order: Orders = raw_order.clone().try_into().unwrap();
        assert_eq!(100.0, order.price, "Error parsing raw order");
        assert_eq!(10.0, order.size, "Error parsing raw order");
        assert_eq!(24, order.num_orders, "Error parsing raw order");

        let product_book: ProductBook = raw_product_book.try_into().unwrap();
        assert_eq!(
            100.0, product_book.asks[0].price,
            "Error parsing raw order book"
        );
        assert_eq!(
            10.0, product_book.asks[0].size,
            "Error parsing raw order book"
        );
        assert_eq!(
            24, product_book.asks[0].num_orders,
            "Error parsing raw order book"
        );
    }

    #[tokio::test]
    async fn test_all_level_1_books() {
        let client = CBProAPI::default();
        let all_prods = client.get_all_products().await.unwrap();

        let level_ones: Vec<_> = all_prods
            .into_iter()
            .map(|x| {
                /*println!("{:?}", x.display_name);*/
                let my_client = client.clone();
                tokio::task::spawn(async move {
                    my_client.get_product_book(x.id, Some(Level::One)).await
                })
            })
            .collect();

        for book in level_ones {
            let book = book.await.unwrap();
            match book {
                Ok(val) => {
                    println!("{:?}", val.sequence)
                }
                Err(err) => {
                    println!("Error: {:?}", err);
                }
            }
        }
    }

    #[tokio::test]
    async fn test_socket() {
        assert_eq!(0b10000000, 128u8);
        assert_eq!(0b00000001, 1u8);
        assert_eq!(0b00000001, 0x01);
        assert_eq!(0b10000000, 0x80);

        assert_eq!(0b00000001u8.leading_zeros(), 7);
    }

    #[tokio::test]
    async fn test_coinbase_websocket_heartbeat() {
        let mut client = CBProAPI::default();

        let subscription = SubscriptionBuilder::new()
            .subscribe_to_heartbeat("ETH-USD".to_string())
            .build();
        let resp = client.subscribe_to_websocket(subscription).await.unwrap();
        println!("{:?}", serde_json::to_string(&resp).unwrap());
    }

    #[tokio::test]
    async fn test_coinbase_websocket_status() {
        let mut client = CBProAPI::default();

        let subscription = SubscriptionBuilder::new().subscribe_to_status().build();
        let resp = client.subscribe_to_websocket(subscription).await.unwrap();
        serde_json::to_string(&resp).unwrap();
        let resp = client.read_websocket().await.unwrap();
        serde_json::to_string(&resp).unwrap();
    }

    #[tokio::test]
    async fn test_coinbase_websocket_ticker() {
        let mut client = CBProAPI::default();

        let subscription = SubscriptionBuilder::new()
            .subscribe_to_ticker("ETH-USD".to_string())
            .build();
        let resp = client.subscribe_to_websocket(subscription).await.unwrap();
        println!("{}", serde_json::to_string(&resp).unwrap());
        let resp = client.read_websocket().await.unwrap();
        println!("{}", serde_json::to_string(&resp).unwrap());
    }

    #[tokio::test]
    async fn test_coinbase_websocket_snapshot() {
        let mut client = CBProAPI::default();

        let subscription = SubscriptionBuilder::new()
            .subscribe_to_snapshot("ETH-USD".to_string())
            .build();
        client.subscribe_to_websocket(subscription).await.unwrap();
        //println!("{}", serde_json::to_string(&resp).unwrap());
        client.read_websocket().await.unwrap();
        //println!("{}", serde_json::to_string(&resp).unwrap());
        let resp = client.read_websocket().await.unwrap();
        println!("{}", serde_json::to_string(&resp).unwrap());
    }

    #[tokio::test]
    async fn test_coinbase_websocket_full() {
        let mut client = CBProAPI::default();

        let subscription = SubscriptionBuilder::new()
            .subscribe_to_full("ETH-USD".to_string())
            .build();
        client.subscribe_to_websocket(subscription).await.unwrap();
        //println!("{}", serde_json::to_string(&resp).unwrap());
        client.read_websocket().await.unwrap();
        //println!("{}", serde_json::to_string(&resp).unwrap());

        for _ in 0..10000 {
            let resp = client.read_websocket().await.unwrap();
            println!("{}", serde_json::to_string(&resp).unwrap());
        }
    }

    // #[tokio::test]
    // async fn test_hmac() {
    //     let client = Client::default();
    //     let account = APIKeyData {
    //         key: KEY.to_string(),
    //         secret: SECRET.to_string(),
    //         passphrase: "blade123".to_string()
    //     };
    //
    //     let this = crate::requests::raw::get_all_accounts_raw(&client, "rust".to_string(), account).await.unwrap();
    //     println!("{}", this.text().await.unwrap());
    // }

    #[tokio::test]
    async fn test_fees() {
        let client = CBProAPI::default();
        let account = APIKeyData {
            key: KEY.to_string(),
            secret: SECRET.to_string(),
            passphrase: "blade123".to_string(),
        };

        let this = client.get_fees(account).await.unwrap();
        println!("{:?}", this);
    }

    // #[tokio::test]
    // async fn test_single_account_raw() {
    //     let client = Client::default();
    //     let account = APIKeyData {
    //         key: KEY.to_string(),
    //         secret: SECRET.to_string(),
    //         passphrase: "blade123".to_string()
    //     };
    //
    //     let this = crate::requests::raw::get_single_account_raw(&client, "rust".to_string(), account, "a48d03fc-d537-4658-9dd6-0c99844e22b8".to_string()).await.unwrap();
    //     let text = this.text().await.unwrap();
    //     assert!(text.contains("{\"id\":\"a48d03fc-d537-4658-9dd6-0c99844e22b8\",\"currency\":\"1INCH\""));
    //     println!("{}", text);
    // }

    // #[tokio::test]
    // async fn test_single_account_holds_raw() {
    //     let client = Client::default();
    //     let account = APIKeyData {
    //         key: KEY.to_string(),
    //         secret: SECRET.to_string(),
    //         passphrase: "blade123".to_string()
    //     };
    //
    //     let this = crate::requests::raw::get_single_account_holds_raw(&client, "rust".to_string(), account, "a0d6c070-c05e-4d01-b6af-d635d6d74917".to_string()).await.unwrap();
    //     println!("{}", this.text().await.unwrap());
    // }

    // #[tokio::test]
    // async fn test_single_account_ledger_raw() {
    //     let client = Client::default();
    //     let account = APIKeyData {
    //         key: KEY.to_string(),
    //         secret: SECRET.to_string(),
    //         passphrase: "blade123".to_string()
    //     };
    //
    //     let mut headers = HeaderMap::new();
    //     headers.insert("CB-AFTER", "10616926141".parse().unwrap());
    //
    //     let this = crate::requests::raw::get_single_account_ledger_raw(
    //         &client,
    //         "rust".to_string(),
    //         account,
    //         "a0d6c070-c05e-4d01-b6af-d635d6d74917".to_string(),
    //         Some(headers),
    //         Some(vec![("after".to_string(), "10619047954".to_string())])
    //     ).await.unwrap();
    //     println!("{:?}", this);
    //     println!("{}", this.text().await.unwrap());
    // }

    // #[tokio::test]
    // async fn test_single_account_ledger() {
    //     let client = Client::default();
    //     let account = APIKeyData {
    //         key: KEY.to_string(),
    //         secret: SECRET.to_string(),
    //         passphrase: "blade123".to_string()
    //     };
    //
    //     let mut headers = HeaderMap::new();
    //     headers.insert("CB-AFTER", "10616926141".parse().unwrap());
    //
    //     let this = crate::requests::get_single_account_ledger(
    //         &client,
    //         "rust".to_string(),
    //         account,
    //         "a0d6c070-c05e-4d01-b6af-d635d6d74917".to_string(),
    //     ).await.unwrap();
    //     println!("{:?}", this);
    //     println!("{:?}", this);
    // }
}
mod websocket_tests {
    use reqwest::{
        Client,
        Method,
        Url,
    };
    use std::time::UNIX_EPOCH;
    use tokio::net::TcpStream;

    use crate::errors::WebsocketError::FrameSize;
    use crate::websocket_lite::{
        Frame,
        WebsocketConnection,
    };

    pub struct FrameInfo<'a> {
        fin: bool,
        opcode: u8,
        masked: bool,
        length: u64,
        mask: &'a [u8],
        payload: &'a [u8],
    }

    pub struct FrameTestCase<'a> {
        raw_data: &'a [u8],
        expected_info: FrameInfo<'a>,
    }

    #[tokio::test]
    async fn websocket_frame_tc1() {
        let test_case = FrameTestCase {
            raw_data: &[0x00, 0x00],
            expected_info: FrameInfo {
                fin: false,
                opcode: 0,
                masked: false,
                length: 0,
                mask: &[],
                payload: &[],
            },
        };

        let test_frame = Frame::new(test_case.raw_data.to_vec()).unwrap();
        assert_eq!(test_case.expected_info.fin, test_frame.fin());
        assert_eq!(test_case.expected_info.length, test_frame.length());
        assert_eq!(test_case.expected_info.masked, test_frame.masked());
        assert_eq!(test_case.expected_info.mask, test_frame.mask());
        assert_eq!(test_case.expected_info.payload, test_frame.payload());
    }

    #[tokio::test]
    async fn websocket_frame_tc2() {
        let test_case = FrameTestCase {
            raw_data: &[0b10000000, 0x00],
            expected_info: FrameInfo {
                fin: true,
                opcode: 0,
                masked: false,
                length: 0,
                mask: &[],
                payload: &[],
            },
        };

        let test_frame = Frame::new(test_case.raw_data.to_vec()).unwrap();
        assert_eq!(test_case.expected_info.fin, test_frame.fin());
        assert_eq!(test_case.expected_info.length, test_frame.length());
        assert_eq!(test_case.expected_info.masked, test_frame.masked());
        assert_eq!(test_case.expected_info.mask, test_frame.mask());
        assert_eq!(test_case.expected_info.payload, test_frame.payload());
    }

    #[tokio::test]
    async fn websocket_frame_tc3() {
        let test_case = FrameTestCase {
            raw_data: &[0b10001111, 0x00],
            expected_info: FrameInfo {
                fin: true,
                opcode: 15,
                masked: false,
                length: 0,
                mask: &[],
                payload: &[],
            },
        };

        let test_frame = Frame::new(test_case.raw_data.to_vec()).unwrap();
        assert_eq!(test_case.expected_info.fin, test_frame.fin());
        assert_eq!(test_case.expected_info.length, test_frame.length());
        assert_eq!(test_case.expected_info.masked, test_frame.masked());
        assert_eq!(test_case.expected_info.mask, test_frame.mask());
        assert_eq!(test_case.expected_info.payload, test_frame.payload());
        assert_eq!(test_case.expected_info.opcode, test_frame.opcode());
    }

    #[tokio::test]
    async fn websocket_frame_tc4() {
        let test_case = FrameTestCase {
            raw_data: &[0b10001111, 0b10000010, 0x00, 0x00, 0x00, 0x00, 0x10, 0xFF],
            expected_info: FrameInfo {
                fin: true,
                opcode: 16,
                masked: true,
                length: 2,
                mask: &[0x00, 0x00, 0x00, 0x00],
                payload: &[0x10, 0xFF],
            },
        };

        let test_frame = Frame::new(test_case.raw_data.to_vec()).unwrap();
        assert_eq!(test_case.expected_info.fin, test_frame.fin());
        assert_eq!(test_case.expected_info.length, test_frame.length());
        assert_eq!(test_case.expected_info.masked, test_frame.masked());
        assert_eq!(test_case.expected_info.mask, test_frame.mask());
        assert_eq!(test_case.expected_info.payload, test_frame.payload());
    }

    #[tokio::test]
    async fn websocket_frame_tc5() {
        let test_case = FrameTestCase {
            raw_data: &[
                0b10001111, 0b11111110, 0b00000000, 0b00000000, 0x00, 0x00, 0x00, 0x00,
            ],
            expected_info: FrameInfo {
                fin: true,
                opcode: 16,
                masked: true,
                length: 0,
                mask: &[0x00, 0x00, 0x00, 0x00],
                payload: &[],
            },
        };

        let test_frame = Frame::new(test_case.raw_data.to_vec()).unwrap();
        assert_eq!(test_case.expected_info.fin, test_frame.fin());
        assert_eq!(test_case.expected_info.length, test_frame.length());
        assert_eq!(test_case.expected_info.masked, test_frame.masked());
        assert_eq!(test_case.expected_info.mask, test_frame.mask());
        assert_eq!(test_case.expected_info.payload, test_frame.payload());
    }

    #[tokio::test]
    async fn websocket_frame_tc6() {
        let test_case = FrameTestCase {
            raw_data: &[
                0b10001111, 0b11111110, 0b00000001, 0b00000000, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00,
            ],
            expected_info: FrameInfo {
                fin: true,
                opcode: 16,
                masked: true,
                length: 256,
                mask: &[0x00, 0x00, 0x00, 0x00],
                payload: &[
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                ],
            },
        };

        let test_frame = Frame::new(test_case.raw_data.to_vec()).unwrap();
        assert_eq!(test_case.expected_info.fin, test_frame.fin());
        assert_eq!(test_case.expected_info.length, test_frame.length());
        assert_eq!(test_case.expected_info.masked, test_frame.masked());
        assert_eq!(test_case.expected_info.mask, test_frame.mask());
        assert_eq!(test_case.expected_info.payload, test_frame.payload());
    }

    #[tokio::test]
    async fn websocket_frame_tc7() {
        let test_case = FrameTestCase {
            raw_data: &[
                0b10001111, 0b11111110, 0b00000001, 0b00000000, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00,
            ],
            expected_info: FrameInfo {
                fin: true,
                opcode: 16,
                masked: true,
                length: 256,
                mask: &[0x00, 0x00, 0x00, 0x00],
                payload: &[
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                ],
            },
        };

        let test_frame = Frame::new(test_case.raw_data.to_vec()).unwrap_err();
        match test_frame {
            FrameSize(err) => {
                assert_eq!(263, err.received_size);
                assert_eq!(264, err.expected_length);
            }
            _ => {
                assert!(false, "Expected Error")
            }
        }
    }

    #[tokio::test]
    async fn websocket_frame_tc8() {
        let test_case = FrameTestCase {
            raw_data: &[
                0b10001111, 0b11111110, 0b00000000, 0b00000000, 0x00, 0x00, 0x00, 0x00, 0x00,
            ],
            expected_info: FrameInfo {
                fin: true,
                opcode: 16,
                masked: true,
                length: 0,
                mask: &[0x00, 0x00, 0x00, 0x00],
                payload: &[0x00],
            },
        };

        let test_frame = Frame::new(test_case.raw_data.to_vec()).unwrap_err();
        match test_frame {
            FrameSize(err) => {
                assert_eq!(9, err.received_size);
                assert_eq!(8, err.expected_length);
            }
            _ => {
                assert!(false, "Expected Error")
            }
        }
    }

    #[tokio::test]
    async fn randomness() {
        let mut generated = [0u64; 256];
        let mut out: u8 = 0;
        let mut sum: u64 = 0;

        for _ in 0..100000 {
            for _ in 0..16 {
                let e1 = std::time::SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos();
                out = ((out << 1) + (e1.count_ones() % 2) as u8) ^ out;
            }
            generated[out as usize] += 1;
            sum += out as u64;
            out = 0;
        }
        println!("{}", sum / 1000);
        println!("{:?}", generated);
    }

    #[tokio::test]
    async fn test_client() {
        let url = Url::parse("https://ws-feed.exchange.coinbase.com/").unwrap();
        let client = Client::new();
        let this = client
            .request(Method::GET, "https://www.test.com")
            .build()
            .unwrap();
        println!("{:?}", this.version());
        println!(
            "{}{}{}{}{}",
            "GET / HTTP/1.1\n",
            "Host: ws-feed.exchange.coinbase.com:443\n",
            "User-Agent: rust\n",
            "Upgrade: websocket\n",
            "Connection: Upgrade\n"
        );

        let mut address = url.socket_addrs(|| None).unwrap()[1];
        address.set_port(443);
        println!("{}", url.domain().unwrap());
        TcpStream::connect(address).await.unwrap();
    }

    #[tokio::test]
    async fn connect_to_coinbase() {
        let url = Url::parse("https://ws-feed.exchange.coinbase.com/").unwrap();

        let sub_req = r#"
{
"type": "subscribe",
"channels": [{ "name": "heartbeat", "product_ids": ["ETH-EUR"] }]
}
"#;
        let mut address = url.socket_addrs(|| None).unwrap()[1];
        println!("{:?}", address);
        address.set_port(443);
        let tcp_stream = TcpStream::connect(address).await.unwrap();

        //let _ = tcp_stream.writable().await;
        //tcp_stream.try_write(https_req.as_bytes()).unwrap();

        // let mut root_store = rustls::RootCertStore::empty();
        // root_store.add_server_trust_anchors(
        //     webpki_roots::TLS_SERVER_ROOTS
        //         .0
        //         .iter()
        //         .map(|ta| {
        //             rustls::OwnedTrustAnchor::from_subject_spki_name_constraints(
        //                 ta.subject,
        //                 ta.spki,
        //                 ta.name_constraints,
        //             )
        //         })
        // );

        let connector = tokio_native_tls::native_tls::TlsConnector::new().unwrap();
        let connector = tokio_native_tls::TlsConnector::from(connector);

        let tls_stream = connector
            .connect("ws-feed.exchange.coinbase.com", tcp_stream)
            .await
            .unwrap();
        let mut websock = WebsocketConnection::new(tls_stream, url).await.unwrap();
        println!(
            "{:?}",
            websock
                .write_string_as_frame(sub_req.to_string())
                .await
                .unwrap()
        );

        let test = websock.read_as_payload_string().await.unwrap();

        println!("{:?}", test);
    }
}
mod api_tests {
    use log::trace;
    use serde::Serialize;

    use crate::data::{
        APIKeyData,
        CBProAPI,
    };
    use crate::datastructs::accounts::LedgerDetail;
    use crate::datastructs::orders::{
        CancelAfter,
        LimitOrder,
        Side,
        Stop,
        TimeInForce,
    };
    use crate::keys::{
        KEY,
        SECRET,
    };

    #[tokio::test]
    async fn get_product_book() {
        let api = CBProAPI::default();

        let resp = api
            .get_product_book("ETH-USD".to_string(), None)
            .await
            .unwrap();

        println!("{:?}", resp);
        //assert!(false);
    }

    #[tokio::test]
    async fn get_all_products() {
        let api = CBProAPI::default();

        let resp = api.get_all_products().await.unwrap();

        println!("{:?}", resp);
        //assert!(false);
    }

    #[tokio::test]
    async fn get_product() {
        let api = CBProAPI::default();

        let resp = api.get_product("ETH-USD".to_string()).await.unwrap();

        println!("{:?}", resp);
        //assert!(false);
    }

    #[tokio::test]
    async fn test_fees() {
        let client = CBProAPI::default();
        let account = APIKeyData {
            key: KEY.to_string(),
            secret: SECRET.to_string(),
            passphrase: "blade123".to_string(),
        };

        let this = client.get_fees(account).await.unwrap();
        println!("{:?}", this);
        //assert!(false);
    }

    #[tokio::test]
    async fn test_ledger() {
        let client = CBProAPI::default();
        let account = APIKeyData {
            key: KEY.to_string(),
            secret: SECRET.to_string(),
            passphrase: "blade123".to_string(),
        };

        let this = client
            .get_account_ledger(account, "a0d6c070-c05e-4d01-b6af-d635d6d74917")
            .await
            .unwrap();
        assert!(this.len() > 1000);
        //assert!(false);
    }

    #[tokio::test]
    async fn test_accounts() {
        let client = CBProAPI::default();
        let account = APIKeyData {
            key: KEY.to_string(),
            secret: SECRET.to_string(),
            passphrase: "blade123".to_string(),
        };

        let this = client.get_accounts(account).await.unwrap();
        let contains = this
            .into_iter()
            .any(|e| e == "a0d6c070-c05e-4d01-b6af-d635d6d74917");
        assert!(contains);
    }

    #[tokio::test]
    async fn test_account() {
        let client = CBProAPI::default();
        let account = APIKeyData {
            key: KEY.to_string(),
            secret: SECRET.to_string(),
            passphrase: "blade123".to_string(),
        };

        let this = client
            .get_account(account, "a0d6c070-c05e-4d01-b6af-d635d6d74917")
            .await
            .unwrap();
        assert_eq!(this, "a0d6c070-c05e-4d01-b6af-d635d6d74917");
        println!("{:?}", this);
    }

    #[tokio::test]
    async fn test_account_holds() {
        let client = CBProAPI::default();
        let account = APIKeyData {
            key: KEY.to_string(),
            secret: SECRET.to_string(),
            passphrase: "blade123".to_string(),
        };

        let this = client
            .get_account_holds(account, "a0d6c070-c05e-4d01-b6af-d635d6d74917")
            .await
            .unwrap();
        assert!(this.len() > 1);
    }

    #[tokio::test]
    async fn test_account_transfers() {
        let client = CBProAPI::default();
        let account = APIKeyData {
            key: KEY.to_string(),
            secret: SECRET.to_string(),
            passphrase: "blade123".to_string(),
        };

        let this = client
            .get_account_transfers(account, "a0d6c070-c05e-4d01-b6af-d635d6d74917")
            .await
            .unwrap();
        println!("{:?}", this);
    }

    #[tokio::test]
    async fn test_wallets() {
        let client = CBProAPI::default();
        let account = APIKeyData {
            key: KEY.to_string(),
            secret: SECRET.to_string(),
            passphrase: "blade123".to_string(),
        };

        let this = client.get_all_wallets(account).await.unwrap();
        println!("{:?}", this);
    }

    #[tokio::test]
    async fn test_currencies() {
        let client = CBProAPI::default();

        let this = client.get_currencies().await.unwrap();
        println!("{:?}", this);
        //assert!(false);
    }

    #[tokio::test]
    async fn test_currency() {
        let client = CBProAPI::default();

        let this = client.get_currency("BTC".to_string()).await.unwrap();
        println!("{:?}", this);
        //assert!(false);
    }

    #[tokio::test]
    async fn test_fills() {
        let client = CBProAPI::default();
        let account = APIKeyData {
            key: KEY.to_string(),
            secret: SECRET.to_string(),
            passphrase: "blade123".to_string(),
        };

        let this = client
            .get_fills(account, None, Some("ETH-USD".to_string()), None)
            .await
            .unwrap();
        println!("{:?}", this);
        //assert!(false);
    }

    #[tokio::test]
    async fn test_orders() {
        let client = CBProAPI::default();
        let account = APIKeyData {
            key: KEY.to_string(),
            secret: SECRET.to_string(),
            passphrase: "blade123".to_string(),
        };

        let this = client
            .get_orders(account, Some("ETH-USD".to_string()), None)
            .await
            .unwrap();
        println!("{:?}", this);
    }

    #[tokio::test]
    async fn limit_order_serialization_test() {
        let order = LimitOrder::new("ETH-USD".to_string(), Side::BUY, 10.0, 1.0);

        println!("{}", serde_json::ser::to_string_pretty(&order).unwrap());
        assert!(false);
    }

    #[tokio::test]
    async fn limit_order_test() {
        simple_logger::init().unwrap();
        let mut client = CBProAPI::default();

        client.client = reqwest::ClientBuilder::default()
            .connection_verbose(true)
            .build()
            .unwrap();

        let account = APIKeyData {
            key: KEY.to_string(),
            secret: SECRET.to_string(),
            passphrase: "blade123".to_string(),
        };

        let order = LimitOrder::new("ETH-USD".to_string(), Side::BUY, 10.0, 1.0)
            .set_time_in_force(Some(TimeInForce::GTT(CancelAfter::Min)));

        let resp = client.create_order(account, order).await.unwrap();
        println!("{:?}", resp);
        assert!(false);
    }

    #[tokio::test]
    async fn limit_order_stop_test() {
        simple_logger::init().unwrap();
        let mut client = CBProAPI::default();

        client.client = reqwest::ClientBuilder::default()
            .connection_verbose(true)
            .build()
            .unwrap();

        let account = APIKeyData {
            key: KEY.to_string(),
            secret: SECRET.to_string(),
            passphrase: "blade123".to_string(),
        };

        let order = LimitOrder::new("ETH-USD".to_string(), Side::BUY, 10.0, 1.0)
            .set_time_in_force(Some(TimeInForce::GTT(CancelAfter::Min)))
            .set_stop(Some(Stop::Entry(9.0)));

        //let resp = client.create_order(account, order).await.unwrap();
        let raw_expected = r#"{"product_id":"ETH-USD","side":"buy","price":"10","size":"1","time_in_force":"GTT","cancel_after":"min","stop":"entry","stop_price":"9"}"#;
        let serialized = serde_json::to_string(&order).unwrap();
        println!("{}", serialized);
        println!("{}", raw_expected);
        //assert!(false);
    }

    #[tokio::test]
    async fn limit_order_stop_send() {
        simple_logger::init().unwrap();
        let mut client = CBProAPI::default();

        client.client = reqwest::ClientBuilder::default()
            .connection_verbose(true)
            .build()
            .unwrap();

        let account = APIKeyData {
            key: KEY.to_string(),
            secret: SECRET.to_string(),
            passphrase: "blade123".to_string(),
        };

        let order = LimitOrder::new("ETH-USD".to_string(), Side::BUY, 10.0, 1.0)
            .set_time_in_force(Some(TimeInForce::GTT(CancelAfter::Min)))
            .set_stop(Some(Stop::Entry(9.0)));

        let resp = client.create_order(account.clone(), order).await.unwrap();
        let resp2 = client
            .get_single_order(account.clone(), resp.id.clone())
            .await
            .map_err(|err| {
                println!("{}", err);
                err
            })
            .unwrap();

        println!("resp: {:?}", resp);
        println!("resp2: {:?}", resp2);
        assert!(false);
    }

    #[tokio::test]
    async fn cfg_mock() {
        let mut client = CBProAPI::default();

        client.client = reqwest::ClientBuilder::default().build().unwrap();

        let account = APIKeyData {
            key: KEY.to_string(),
            secret: SECRET.to_string(),
            passphrase: "blade123".to_string(),
        };

        let accounts = client
            .get_account_ledger(account, "a0d6c070-c05e-4d01-b6af-d635d6d74917")
            .await
            .unwrap();

        accounts
            .iter()
            .filter(|x| match x.details {
                LedgerDetail::Match(_) => false,
                LedgerDetail::Transfer(_) => false,
                LedgerDetail::Fee(_) => true,
                LedgerDetail::Rebate(_) => true,
                LedgerDetail::Conversion(_) => true,
            })
            .for_each(|x| println!("{}", x));
    }
}
mod frame {
    #[tokio::test]
    async fn websocket_frame_tc1() {
        SimpleLogger::new().with_level(LevelFilter::Trace).init();
        let test_case = FrameTestCase {
            raw_data: &[0x00, 0x00],
            expected_info: FrameInfo {
                fin: false,
                opcode: 0,
                masked: false,
                length: 0,
                mask: &[],
                payload: &[],
            },
        };

        let test_frame = Frame::new(test_case.raw_data.to_vec()).unwrap();
        assert_eq!(test_case.expected_info.fin, test_frame.fin());
        assert_eq!(test_case.expected_info.length, test_frame.length());
        assert_eq!(test_case.expected_info.masked, test_frame.masked());
        assert_eq!(test_case.expected_info.mask, test_frame.mask());
        assert_eq!(test_case.expected_info.payload, test_frame.payload());
    }

    #[tokio::test]
    async fn websocket_frame_tc2() {
        SimpleLogger::new().with_level(LevelFilter::Trace).init();
        let test_case = FrameTestCase {
            raw_data: &[0b10000000, 0x00],
            expected_info: FrameInfo {
                fin: true,
                opcode: 0,
                masked: false,
                length: 0,
                mask: &[],
                payload: &[],
            },
        };

        let test_frame = Frame::new(test_case.raw_data.to_vec()).unwrap();
        assert_eq!(test_case.expected_info.fin, test_frame.fin());
        assert_eq!(test_case.expected_info.length, test_frame.length());
        assert_eq!(test_case.expected_info.masked, test_frame.masked());
        assert_eq!(test_case.expected_info.mask, test_frame.mask());
        assert_eq!(test_case.expected_info.payload, test_frame.payload());
    }

    #[tokio::test]
    async fn websocket_frame_tc3() {
        SimpleLogger::new().with_level(LevelFilter::Trace).init();
        let test_case = FrameTestCase {
            raw_data: &[0b10001111, 0x00],
            expected_info: FrameInfo {
                fin: true,
                opcode: 15,
                masked: false,
                length: 0,
                mask: &[],
                payload: &[],
            },
        };

        let test_frame = Frame::new(test_case.raw_data.to_vec()).unwrap();
        assert_eq!(test_case.expected_info.fin, test_frame.fin());
        assert_eq!(test_case.expected_info.length, test_frame.length());
        assert_eq!(test_case.expected_info.masked, test_frame.masked());
        assert_eq!(test_case.expected_info.mask, test_frame.mask());
        assert_eq!(test_case.expected_info.payload, test_frame.payload());
        assert_eq!(test_case.expected_info.opcode, test_frame.opcode());
    }

    #[tokio::test]
    async fn websocket_frame_tc4() {
        SimpleLogger::new().with_level(LevelFilter::Trace).init();
        let test_case = FrameTestCase {
            raw_data: &[0b10001111, 0b10000010, 0x00, 0x00, 0x00, 0x00, 0x10, 0xFF],
            expected_info: FrameInfo {
                fin: true,
                opcode: 16,
                masked: true,
                length: 2,
                mask: &[0x00, 0x00, 0x00, 0x00],
                payload: &[0x10, 0xFF],
            },
        };

        let test_frame = Frame::new(test_case.raw_data.to_vec()).unwrap();
        assert_eq!(test_case.expected_info.fin, test_frame.fin());
        assert_eq!(test_case.expected_info.length, test_frame.length());
        assert_eq!(test_case.expected_info.masked, test_frame.masked());
        assert_eq!(test_case.expected_info.mask, test_frame.mask());
        assert_eq!(test_case.expected_info.payload, test_frame.payload());
    }

    #[tokio::test]
    async fn websocket_frame_tc5() {
        SimpleLogger::new().with_level(LevelFilter::Trace).init();
        let test_case = FrameTestCase {
            raw_data: &[
                0b10001111, 0b11111110, 0b00000000, 0b00000000, 0x00, 0x00, 0x00, 0x00,
            ],
            expected_info: FrameInfo {
                fin: true,
                opcode: 16,
                masked: true,
                length: 0,
                mask: &[0x00, 0x00, 0x00, 0x00],
                payload: &[],
            },
        };

        let test_frame = Frame::new(test_case.raw_data.to_vec()).unwrap();
        assert_eq!(test_case.expected_info.fin, test_frame.fin());
        assert_eq!(test_case.expected_info.length, test_frame.length());
        assert_eq!(test_case.expected_info.masked, test_frame.masked());
        assert_eq!(test_case.expected_info.mask, test_frame.mask());
        assert_eq!(test_case.expected_info.payload, test_frame.payload());
    }

    #[tokio::test]
    async fn websocket_frame_tc6() {
        SimpleLogger::new().with_level(LevelFilter::Trace).init();
        let test_case = FrameTestCase {
            raw_data: &[
                0b10001111, 0b11111110, 0b00000001, 0b00000000, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00,
            ],
            expected_info: FrameInfo {
                fin: true,
                opcode: 16,
                masked: true,
                length: 256,
                mask: &[0x00, 0x00, 0x00, 0x00],
                payload: &[
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                ],
            },
        };

        let test_frame = Frame::new(test_case.raw_data.to_vec()).unwrap();
        assert_eq!(test_case.expected_info.fin, test_frame.fin());
        assert_eq!(test_case.expected_info.length, test_frame.length());
        assert_eq!(test_case.expected_info.masked, test_frame.masked());
        assert_eq!(test_case.expected_info.mask, test_frame.mask());
        assert_eq!(test_case.expected_info.payload, test_frame.payload());
    }

    #[tokio::test]
    async fn websocket_frame_tc7() {
        SimpleLogger::new().with_level(LevelFilter::Trace).init();
        let test_case = FrameTestCase {
            raw_data: &[
                0b10001111, 0b11111110, 0b00000001, 0b00000000, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00,
            ],
            expected_info: FrameInfo {
                fin: true,
                opcode: 16,
                masked: true,
                length: 256,
                mask: &[0x00, 0x00, 0x00, 0x00],
                payload: &[
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                ],
            },
        };

        let test_frame = Frame::new(test_case.raw_data.to_vec()).unwrap_err();
        match test_frame {
            FrameSize(err) => {
                assert_eq!(263, err.received_size);
                assert_eq!(264, err.expected_length);
            }
            _ => {
                assert!(false, "Expected Error")
            }
        }
    }

    #[tokio::test]
    async fn websocket_frame_tc8() {
        SimpleLogger::new().with_level(LevelFilter::Trace).init();
        let test_case = FrameTestCase {
            raw_data: &[
                0b10001111, 0b11111110, 0b00000000, 0b00000000, 0x00, 0x00, 0x00, 0x00, 0x00,
            ],
            expected_info: FrameInfo {
                fin: true,
                opcode: 16,
                masked: true,
                length: 0,
                mask: &[0x00, 0x00, 0x00, 0x00],
                payload: &[0x00],
            },
        };

        let test_frame = Frame::new(test_case.raw_data.to_vec()).unwrap_err();
        match test_frame {
            FrameSize(err) => {
                assert_eq!(9, err.received_size);
                assert_eq!(8, err.expected_length);
            }
            _ => {
                assert!(false, "Expected Error")
            }
        }
    }
}
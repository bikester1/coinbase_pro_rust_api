#[derive(Debug)]
pub struct Frame {
    byte_array: Vec<u8>,
    length_measured: bool,
    length: u64,
    payload_offset: usize,
}

#[allow(dead_code)]
impl Frame {
    const FIN_MASK: u8 = 0b10000000;
    const OPCODE_MASK: u8 = 0b00001111;
    const MASKED_MASK: u8 = 0b10000000;
    const LENGTH_1_MASK: u8 = 0b01111111;

    const OPTION_FINAL: u8 = 0b10000000;
    const OPTION_MASKED: u8 = 0b10000000;

    /// Opcodes
    ///      |Opcode  | Meaning                             | Reference |
    ///     -+--------+-------------------------------------+-----------|
    ///      | 0      | Continuation Frame                  | RFC 6455  |
    ///     -+--------+-------------------------------------+-----------|
    ///      | 1      | Text Frame                          | RFC 6455  |
    ///     -+--------+-------------------------------------+-----------|
    ///      | 2      | Binary Frame                        | RFC 6455  |
    ///     -+--------+-------------------------------------+-----------|
    ///      | 8      | Connection Close Frame              | RFC 6455  |
    ///     -+--------+-------------------------------------+-----------|
    ///      | 9      | Ping Frame                          | RFC 6455  |
    ///     -+--------+-------------------------------------+-----------|
    ///      | 10     | Pong Frame                          | RFC 6455  |
    ///     -+--------+-------------------------------------+-----------|
    const OPCODE_CONTINUATION: u8 = 00;
    const OPCODE_TEXT: u8 = 01;
    const OPCODE_BINARY: u8 = 02;
    const OPCODE_CLOSE: u8 = 08;
    const OPCODE_PING: u8 = 09;
    const OPCODE_PONG: u8 = 10;

    pub fn from_payload_masked(payload: String) -> Self {
        // Initialize a byte vec with the contents of the payload
        let payload_bytes = payload.as_bytes().to_vec();

        // Generate a random mask
        let mut mask = [0u8; 4];
        rand::rngs::StdRng::from_entropy().fill_bytes(&mut mask);

        // XOR each byte of payload with it's index remainder 4th byte of the mask bytes.
        // must guarantee that mask is 4 bytes or this will panic.
        let mut masked_payload = Self::apply_mask(payload_bytes, mask.to_vec()).unwrap();

        // set length bytes.
        // < 126                    -> 1 byte   [Len]
        // 126 >= len < u16::MAX    -> 3 bytes  [126, Len[0], Len[1]]
        // u16::MAX < len           -> 9 bytes  [127, Len[0], Len[1], Len[2], Len[3], Len[4], Len[5], Len[6], Len[7]]
        //
        // These need to be seperate computations because the u16 and u64 case can't be merged without unsafe code.
        let mut header_bytes = if masked_payload.len() < 126 {
            // Length is just the byte count of the payload if the payload is < 126 bytes.
            vec![0u8, masked_payload.len() as u8]
        } else if masked_payload.len() < u16::MAX as usize {
            // Payload length as a 2 byte int
            let len = masked_payload.len() as u16;

            // Length section 1 is set to 126
            let mut vec = vec![0u8, 126u8];

            // Length section 2 is len as bigendian bytes
            let mut len_bytes = len.to_be_bytes().to_vec();

            // Append the 2 bytes for the length
            vec.append(&mut len_bytes);
            vec
        } else {
            // Payload length as a 4 byte int
            let len = masked_payload.len() as u64;

            // Length section 1 is set to 127
            let mut vec = vec![0u8, 127u8];

            // Length section 2 is len as bigendian bytes
            let mut len_bytes = len.to_be_bytes().to_vec();

            // Append the 4 bytes for the length
            vec.append(&mut len_bytes);
            vec
        };

        let len: u64 = masked_payload.len() as u64;
        let payload_offset = header_bytes.len() + 4;
        header_bytes[0] = Self::OPTION_FINAL | Self::OPCODE_TEXT;
        header_bytes[1] = Self::OPTION_MASKED | header_bytes[1];
        header_bytes.append(&mut mask.to_vec());
        header_bytes.append(&mut masked_payload);

        Frame {
            byte_array: header_bytes,
            length_measured: true,
            length: len,
            payload_offset,
        }
    }

    pub fn seed(self: Self) -> Vec<u8> {
        todo!()
    }

    pub fn into_vec(self: Self) -> Vec<u8> {
        self.byte_array
    }

    pub fn new(bytes: Vec<u8>) -> Result<Self, WebsocketError> {
        let mut expected: usize = 2;
        let len = bytes.len();
        Self::check_size(len, expected)?;

        let mut initial_frame = Frame {
            byte_array: bytes.clone(),
            length_measured: false,
            length: 0,
            payload_offset: 0,
        };

        let first_len_comp = initial_frame.byte_array[1] & Self::LENGTH_1_MASK;
        if first_len_comp < 126 {
            initial_frame.length = first_len_comp as u64;
        } else if first_len_comp == 126 {
            // if len is 126 we need to ensure there are at least 4 bytes to continue
            expected += 2;
            Self::check_size(len, expected)?;

            initial_frame.length = ((initial_frame.byte_array[2] as u16)
                << 8 + initial_frame.byte_array[3] as u16)
                as u64;
        } else {
            // if we have a len of 127 we need at least 10 bytes to continue
            expected += 8;
            Self::check_size(len, expected)?;

            initial_frame.length = Self::merge_8_bytes(&initial_frame.byte_array[2..10]);
        }

        if initial_frame.masked() {
            expected += 4
        }
        initial_frame.payload_offset = expected;
        expected += initial_frame.length as usize;

        Self::check_size_abs(len, expected)?;

        Ok(initial_frame)
        //let frame_size = 2 + initial_frame.length()
    }

    pub async fn new_from_stream<T: AsyncReadExt + Unpin + Send>(
        stream: &mut T,
    ) -> Result<Self, WebsocketError> {
        // Initialize an empty frame struct
        // This struct is invalid until returned
        let mut initial_frame = Frame {
            byte_array: Vec::new(),
            length_measured: false,
            length: 0,
            payload_offset: 0,
        };

        // load first 2 bytes into frame
        let mut buf = [0u8; 2];
        stream
            .read_exact(&mut buf)
            .await
            .map_err(|_| UnexpectedEnd {
                expected_length: 2,
                received_size: 0,
                #[feature(backtrace)]
                backtrace: Backtrace::capture(),
            })?;

        // Append bytes read
        initial_frame.byte_array.append(&mut buf.to_vec());

        // Length as indicated by first length byte
        let first_len_comp = initial_frame.byte_array[1] & Self::LENGTH_1_MASK;

        // Depending on len read next 2 or 8 bytes
        if first_len_comp < 126 {
            // Do not read additional bytes and set this frames expected lenght
            initial_frame.length = first_len_comp as u64;

        // if len is 126 we need to ensure there are at least 4 bytes to continue
        } else if first_len_comp == 126 {
            // load 2 more bytes into frame
            let mut buf = [0u8; 2];
            stream
                .read_exact(&mut buf)
                .await
                .map_err(|_| UnexpectedEnd {
                    expected_length: 4,
                    received_size: 2,
                    #[feature(backtrace)]
                    backtrace: Backtrace::capture(),
                })?;
            initial_frame.byte_array.append(&mut buf.to_vec());

            initial_frame.length = (((initial_frame.byte_array[2] as u16) << 8)
                + initial_frame.byte_array[3] as u16) as u64;
        } else {
            // if we have a len of 127 we need at least 10 bytes to continue

            // load 8 bytes into frame
            let mut buf = [0u8; 8];
            stream
                .read_exact(&mut buf)
                .await
                .map_err(|_| UnexpectedEnd {
                    expected_length: 10,
                    received_size: 2,
                    #[feature(backtrace)]
                    backtrace: Backtrace::capture(),
                })?;
            initial_frame.byte_array.append(&mut buf.to_vec());

            initial_frame.length = Self::merge_8_bytes(&initial_frame.byte_array[2..10]);
        }

        if initial_frame.masked() {
            // load 4 bytes for the mask into frame
            let mut buf = [0u8; 4];
            stream
                .read_exact(&mut buf)
                .await
                .map_err(|_| UnexpectedEnd {
                    expected_length: initial_frame.byte_array.len() + 4,
                    received_size: initial_frame.byte_array.len(),
                    #[feature(backtrace)]
                    backtrace: Backtrace::capture(),
                })?;
            initial_frame.byte_array.append(&mut buf.to_vec());
        }
        initial_frame.payload_offset = initial_frame.byte_array.len();

        // load the payload into frame
        let mut buf = Vec::new();
        buf.resize(initial_frame.length as usize, 0);
        stream
            .read_exact(buf.as_mut_slice())
            .await
            .map_err(|_| UnexpectedEnd {
                expected_length: initial_frame.byte_array.len() + buf.len(),
                received_size: initial_frame.byte_array.len(),
                #[feature(backtrace)]
                backtrace: Backtrace::capture(),
            })?;
        initial_frame.byte_array.append(&mut buf.to_vec());

        Ok(initial_frame)
        //let frame_size = 2 + initial_frame.length()
    }

    fn check_size(len: usize, expected: usize) -> Result<(), WebsocketError> {
        if len < expected {
            Err(UnexpectedEnd {
                expected_length: expected,
                received_size: len,
                #[feature(backtrace)]
                backtrace: Backtrace::capture(),
            }
            .into())
        } else {
            Ok(())
        }
    }

    fn check_size_abs(len: usize, expected: usize) -> Result<(), WebsocketError> {
        if len != expected {
            Err(UnexpectedEnd {
                expected_length: expected,
                received_size: len,
                #[feature(backtrace)]
                backtrace: Backtrace::capture(),
            }
            .into())
        } else {
            Ok(())
        }
    }

    pub fn fin(self: &Self) -> bool {
        self.byte_array[0] & Self::FIN_MASK == Self::OPTION_FINAL
    }

    pub fn opcode(self: &Self) -> u8 {
        self.byte_array[0] & Self::OPCODE_MASK
    }

    pub fn masked(self: &Self) -> bool {
        self.byte_array[1] & Self::MASKED_MASK == Self::OPTION_MASKED
    }

    pub fn length(self: &Self) -> u64 {
        self.length
    }

    pub fn payload(self: &Self) -> Vec<u8> {
        self.payload_ref().to_vec()
    }

    pub fn payload_ref(self: &Self) -> &[u8] {
        &self.byte_array[self.payload_offset..]
    }

    pub fn unmasked_payload(self: &Self) -> Vec<u8> {
        let mask = self.mask().to_vec();
        if !self.masked() || mask.len() != 4 {
            self.payload()
        } else {
            Self::apply_mask(self.payload(), mask).unwrap()
        }
    }

    pub fn apply_mask(payload: Vec<u8>, mask: Vec<u8>) -> Result<Vec<u8>, WebsocketError> {
        if mask.len() != 4 {
            return Err(IncorrectMaskSize {
                expected_length: 4,
                received_size: mask.len(),
            }
            .into());
        }

        Ok(payload
            .into_iter()
            .enumerate()
            .map(|(i, val)| val ^ mask[i % 4])
            .collect::<Vec<u8>>())
    }

    pub fn mask(self: &Self) -> &[u8] {
        if self.masked() {
            &self.byte_array[self.payload_offset - 4..self.payload_offset]
        } else {
            &[]
        }
    }

    #[inline]
    fn merge_8_bytes(bytes: &[u8]) -> u64 {
        assert_eq!(8, bytes.len());
        let mut ret_val = 0u64;
        for byte in bytes {
            ret_val = ret_val << 8;
            ret_val += byte.clone() as u64;
        }
        ret_val
    }
}
use crate::types::{
    properties::*, ConnectAckPacket, ConnectPacket, ConnectReason, DecodeError, DisconnectPacket,
    DisconnectReason, FinalWill, Packet, PacketType, QoS,
};
use bytes::{Buf, BytesMut};
use std::{convert::TryFrom, io::Cursor};

macro_rules! return_if_none {
    ($x: expr) => {{
        let string_opt = $x;
        if string_opt.is_none() {
            return Ok(None);
        }

        string_opt.unwrap()
    }};
}

macro_rules! require_length {
    ($bytes: expr, $len: expr) => {{
        if $bytes.remaining() < $len {
            return Ok(None);
        }
    }};
}

macro_rules! read_u8 {
    ($bytes: expr) => {{
        if !$bytes.has_remaining() {
            return Ok(None);
        }

        $bytes.get_u8()
    }};
}

macro_rules! read_u16 {
    ($bytes: expr) => {{
        if $bytes.remaining() < 2 {
            return Ok(None);
        }

        $bytes.get_u16()
    }};
}

macro_rules! read_u32 {
    ($bytes: expr) => {{
        if $bytes.remaining() < 4 {
            return Ok(None);
        }

        $bytes.get_u32()
    }};
}

macro_rules! read_variable_int {
    ($bytes: expr) => {{
        return_if_none!(decode_variable_int($bytes)?)
    }};
}

macro_rules! read_string {
    ($bytes: expr) => {{
        return_if_none!(decode_string($bytes)?)
    }};
}

macro_rules! read_binary_data {
    ($bytes: expr) => {{
        return_if_none!(decode_binary_data($bytes)?)
    }};
}

macro_rules! read_string_pair {
    ($bytes: expr) => {{
        let string_key = read_string!($bytes);
        let string_value = read_string!($bytes);

        (string_key, string_value)
    }};
}

macro_rules! read_property {
    ($bytes: expr) => {{
        let property_id = read_variable_int!($bytes);
        return_if_none!(decode_property(property_id, $bytes)?)
    }};
}

fn decode_variable_int(bytes: &mut Cursor<&mut BytesMut>) -> Result<Option<u32>, DecodeError> {
    let mut multiplier = 1;
    let mut value: u32 = 0;

    loop {
        let encoded_byte = read_u8!(bytes);

        value += ((encoded_byte & 0b0111_1111) as u32) * multiplier;
        multiplier *= 128;

        if multiplier > (128 * 128 * 128) {
            return Err(DecodeError::InvalidRemainingLength);
        }

        if encoded_byte & 0b1000_0000 == 0b0000_0000 {
            break;
        }
    }

    Ok(Some(value))
}

fn decode_string(bytes: &mut Cursor<&mut BytesMut>) -> Result<Option<String>, DecodeError> {
    let str_size_bytes = read_u16!(bytes) as usize;

    require_length!(bytes, str_size_bytes);

    let position = bytes.position() as usize;

    // TODO - Use Cow<str> and from_utf8_lossy later for less copying
    match String::from_utf8(bytes.get_ref()[position..(position + str_size_bytes)].into()) {
        Ok(string) => {
            bytes.advance(str_size_bytes);
            Ok(Some(string))
        },
        Err(_) => Err(DecodeError::InvalidUtf8),
    }
}

fn decode_binary_data(bytes: &mut Cursor<&mut BytesMut>) -> Result<Option<Vec<u8>>, DecodeError> {
    let data_size_bytes = read_u16!(bytes) as usize;
    require_length!(bytes, data_size_bytes);

    let position = bytes.position() as usize;

    Ok(Some(bytes.get_ref()[position..(position + data_size_bytes)].into()))
}

fn decode_property(
    property_id: u32,
    bytes: &mut Cursor<&mut BytesMut>,
) -> Result<Option<Property>, DecodeError> {
    match property_id {
        1 => {
            let format_indicator = read_u8!(bytes);
            Ok(Some(Property::PayloadFormatIndicator(PayloadFormatIndicator(format_indicator))))
        },
        2 => {
            let message_expiry_interval = read_u32!(bytes);
            Ok(Some(Property::MessageExpiryInterval(MessageExpiryInterval(
                message_expiry_interval,
            ))))
        },
        3 => {
            let content_type = read_string!(bytes);
            Ok(Some(Property::ContentType(ContentType(content_type))))
        },
        8 => {
            let response_topic = read_string!(bytes);
            Ok(Some(Property::RepsonseTopic(RepsonseTopic(response_topic))))
        },
        9 => {
            let correlation_data = read_binary_data!(bytes);
            Ok(Some(Property::CorrelationData(CorrelationData(correlation_data))))
        },
        11 => {
            let subscription_identifier = read_u32!(bytes);
            Ok(Some(Property::SubscriptionIdentifier(SubscriptionIdentifier(
                subscription_identifier,
            ))))
        },
        17 => {
            let session_expiry_interval = read_u32!(bytes);
            Ok(Some(Property::SessionExpiryInterval(SessionExpiryInterval(
                session_expiry_interval,
            ))))
        },
        18 => {
            let assigned_client_identifier = read_string!(bytes);
            Ok(Some(Property::AssignedClientIdentifier(AssignedClientIdentifier(
                assigned_client_identifier,
            ))))
        },
        19 => {
            let server_keep_alive = read_u16!(bytes);
            Ok(Some(Property::ServerKeepAlive(ServerKeepAlive(server_keep_alive))))
        },
        21 => {
            let authentication_method = read_string!(bytes);
            Ok(Some(Property::AuthenticationMethod(AuthenticationMethod(authentication_method))))
        },
        22 => {
            let authentication_data = read_binary_data!(bytes);
            Ok(Some(Property::AuthenticationData(AuthenticationData(authentication_data))))
        },
        23 => {
            let request_problem_information = read_u8!(bytes);
            Ok(Some(Property::RequestProblemInformation(RequestProblemInformation(
                request_problem_information,
            ))))
        },
        24 => {
            let will_delay_interval = read_u32!(bytes);
            Ok(Some(Property::WillDelayInterval(WillDelayInterval(will_delay_interval))))
        },
        25 => {
            let request_response_information = read_u8!(bytes);
            Ok(Some(Property::RequestResponseInformation(RequestResponseInformation(
                request_response_information,
            ))))
        },
        26 => {
            let response_information = read_string!(bytes);
            Ok(Some(Property::ResponseInformation(ResponseInformation(response_information))))
        },
        28 => {
            let server_reference = read_string!(bytes);
            Ok(Some(Property::ServerReference(ServerReference(server_reference))))
        },
        31 => {
            let reason_string = read_string!(bytes);
            Ok(Some(Property::ReasonString(ReasonString(reason_string))))
        },
        33 => {
            let receive_maximum = read_u16!(bytes);
            Ok(Some(Property::ReceiveMaximum(ReceiveMaximum(receive_maximum))))
        },
        34 => {
            let topic_alias_maximum = read_u16!(bytes);
            Ok(Some(Property::TopicAliasMaximum(TopicAliasMaximum(topic_alias_maximum))))
        },
        35 => {
            let topic_alias = read_u16!(bytes);
            Ok(Some(Property::TopicAlias(TopicAlias(topic_alias))))
        },
        36 => {
            let qos_byte = read_u8!(bytes);
            let qos = QoS::try_from(qos_byte)?;

            Ok(Some(Property::MaximumQos(MaximumQos(qos))))
        },
        37 => {
            let retain_available = read_u8!(bytes);
            Ok(Some(Property::RetainAvailable(RetainAvailable(retain_available))))
        },
        38 => {
            let (key, value) = read_string_pair!(bytes);
            Ok(Some(Property::UserProperty(UserProperty(key, value))))
        },
        39 => {
            let maximum_packet_size = read_u32!(bytes);
            Ok(Some(Property::MaximumPacketSize(MaximumPacketSize(maximum_packet_size))))
        },
        40 => {
            let wildcard_subscription_available = read_u8!(bytes);
            Ok(Some(Property::WildcardSubscriptionAvailable(WildcardSubscriptionAvailable(
                wildcard_subscription_available,
            ))))
        },
        41 => {
            let subscription_identifier_available = read_u8!(bytes);
            Ok(Some(Property::SubscriptionIdentifierAvailable(SubscriptionIdentifierAvailable(
                subscription_identifier_available,
            ))))
        },
        42 => {
            let shared_subscription_available = read_u8!(bytes);
            Ok(Some(Property::SharedSubscriptionAvailable(SharedSubscriptionAvailable(
                shared_subscription_available,
            ))))
        },
        _ => Err(DecodeError::InvalidPropertyId),
    }
}

fn decode_properties<F: FnMut(Property)>(
    bytes: &mut Cursor<&mut BytesMut>,
    mut closure: F,
) -> Result<Option<()>, DecodeError> {
    let property_length = read_variable_int!(bytes);

    if property_length == 0 {
        return Ok(Some(()));
    }

    require_length!(bytes, property_length as usize);

    let start_cursor_pos = bytes.position();

    loop {
        let cursor_pos = bytes.position();

        if cursor_pos - start_cursor_pos >= property_length as u64 {
            break;
        }

        let property = read_property!(bytes);
        closure(property);
    }

    Ok(Some(()))
}

fn decode_connect(bytes: &mut Cursor<&mut BytesMut>) -> Result<Option<Packet>, DecodeError> {
    let protocol_name = read_string!(bytes);
    let protocol_level = read_u8!(bytes);
    let connect_flags = read_u8!(bytes);
    let keep_alive = read_u16!(bytes);

    let mut session_expiry_interval = None;
    let mut receive_maximum = None;
    let mut maximum_packet_size = None;
    let mut topic_alias_maximum = None;
    let mut request_response_information = None;
    let mut request_problem_information = None;
    let mut user_properties = vec![];
    let mut authentication_method = None;
    let mut authentication_data = None;

    return_if_none!(decode_properties(bytes, |property| {
        match property {
            Property::SessionExpiryInterval(p) => session_expiry_interval = Some(p),
            Property::ReceiveMaximum(p) => receive_maximum = Some(p),
            Property::MaximumPacketSize(p) => maximum_packet_size = Some(p),
            Property::TopicAliasMaximum(p) => topic_alias_maximum = Some(p),
            Property::RequestResponseInformation(p) => request_response_information = Some(p),
            Property::RequestProblemInformation(p) => request_problem_information = Some(p),
            Property::UserProperty(p) => user_properties.push(p),
            Property::AuthenticationMethod(p) => authentication_method = Some(p),
            Property::AuthenticationData(p) => authentication_data = Some(p),
            _ => {}, // Invalid property for packet
        }
    })?);

    // Start payload
    let clean_start = 0b0000_0010 & connect_flags == 0b0000_0010;
    let has_will = 0b0000_0100 & connect_flags == 0b0000_0100;
    let will_qos_val = (0b0001_1000 & connect_flags) >> 3;
    let will_qos = QoS::try_from(will_qos_val)?;
    let retain_will = 0b0010_0000 & connect_flags == 0b0010_0000;
    let has_password = 0b0100_0000 & connect_flags == 0b0100_0000;
    let has_user_name = 0b1000_0000 & connect_flags == 0b1000_0000;

    let client_id = read_string!(bytes);

    let will = if has_will {
        let mut will_delay_interval = None;
        let mut payload_format_indicator = None;
        let mut message_expiry_interval = None;
        let mut content_type = None;
        let mut response_topic = None;
        let mut correlation_data = None;
        let mut user_properties = vec![];

        return_if_none!(decode_properties(bytes, |property| {
            match property {
                Property::WillDelayInterval(p) => will_delay_interval = Some(p),
                Property::PayloadFormatIndicator(p) => payload_format_indicator = Some(p),
                Property::MessageExpiryInterval(p) => message_expiry_interval = Some(p),
                Property::ContentType(p) => content_type = Some(p),
                Property::RepsonseTopic(p) => response_topic = Some(p),
                Property::CorrelationData(p) => correlation_data = Some(p),
                Property::UserProperty(p) => user_properties.push(p),
                _ => {}, // Invalid property for packet
            }
        })?);

        let topic = read_string!(bytes);
        let payload = read_binary_data!(bytes);

        Some(FinalWill {
            topic,
            payload,
            qos: will_qos,
            should_retain: retain_will,
            will_delay_interval,
            payload_format_indicator,
            message_expiry_interval,
            content_type,
            response_topic,
            correlation_data,
            user_properties,
        })
    } else {
        None
    };

    let mut user_name = None;
    let mut password = None;

    if has_user_name {
        user_name = Some(read_string!(bytes));
    }

    if has_password {
        password = Some(read_string!(bytes));
    }

    let packet = ConnectPacket {
        protocol_name,
        protocol_level,
        clean_start,
        keep_alive,
        session_expiry_interval,
        receive_maximum,
        maximum_packet_size,
        topic_alias_maximum,
        request_response_information,
        request_problem_information,
        user_properties,
        authentication_method,
        authentication_data,
        client_id,
        will,
        user_name,
        password,
    };

    Ok(Some(Packet::Connect(packet)))
}

fn decode_connect_ack(bytes: &mut Cursor<&mut BytesMut>) -> Result<Option<Packet>, DecodeError> {
    let flags = read_u8!(bytes);
    let session_present = (flags & 0b0000_0001) == 0b0000_0001;

    let reason_code_byte = read_u8!(bytes);
    let reason = ConnectReason::try_from(reason_code_byte)?;

    let mut session_expiry_interval = None;
    let mut receive_maximum = None;
    let mut maximum_qos = None;
    let mut retain_available = None;
    let mut maximum_packet_size = None;
    let mut assigned_client_identifier = None;
    let mut topic_alias_maximum = None;
    let mut reason_string = None;
    let mut user_properties = vec![];
    let mut wildcard_subscription_available = None;
    let mut subscription_identifiers_available = None;
    let mut shared_subscription_available = None;
    let mut server_keep_alive = None;
    let mut response_information = None;
    let mut server_reference = None;
    let mut authentication_method = None;
    let mut authentication_data = None;

    return_if_none!(decode_properties(bytes, |property| {
        match property {
            Property::SessionExpiryInterval(p) => session_expiry_interval = Some(p),
            Property::ReceiveMaximum(p) => receive_maximum = Some(p),
            Property::MaximumQos(p) => maximum_qos = Some(p),
            Property::RetainAvailable(p) => retain_available = Some(p),
            Property::MaximumPacketSize(p) => maximum_packet_size = Some(p),
            Property::AssignedClientIdentifier(p) => assigned_client_identifier = Some(p),
            Property::TopicAliasMaximum(p) => topic_alias_maximum = Some(p),
            Property::ReasonString(p) => reason_string = Some(p),
            Property::UserProperty(p) => user_properties.push(p),
            Property::WildcardSubscriptionAvailable(p) => wildcard_subscription_available = Some(p),
            Property::SubscriptionIdentifierAvailable(p) => {
                subscription_identifiers_available = Some(p)
            },
            Property::SharedSubscriptionAvailable(p) => shared_subscription_available = Some(p),
            Property::ServerKeepAlive(p) => server_keep_alive = Some(p),
            Property::ResponseInformation(p) => response_information = Some(p),
            Property::ServerReference(p) => server_reference = Some(p),
            Property::AuthenticationMethod(p) => authentication_method = Some(p),
            Property::AuthenticationData(p) => authentication_data = Some(p),
            _ => {}, // Invalid property for packet
        }
    })?);

    let packet = ConnectAckPacket {
        session_present,
        reason,
        session_expiry_interval,
        receive_maximum,
        maximum_qos,
        retain_available,
        maximum_packet_size,
        assigned_client_identifier,
        topic_alias_maximum,
        reason_string,
        user_properties,
        wildcard_subscription_available,
        subscription_identifiers_available,
        shared_subscription_available,
        server_keep_alive,
        response_information,
        server_reference,
        authentication_method,
        authentication_data,
    };

    Ok(Some(Packet::ConnectAck(packet)))
}

fn decode_disconnect(
    bytes: &mut Cursor<&mut BytesMut>,
    remaining_packet_length: u32,
) -> Result<Option<Packet>, DecodeError> {
    let reason_code_byte = read_u8!(bytes);
    let reason = DisconnectReason::try_from(reason_code_byte)?;

    let mut session_expiry_interval = None;
    let mut reason_string = None;
    let mut user_properties = vec![];
    let mut server_reference = None;

    if remaining_packet_length >= 2 {
        return_if_none!(decode_properties(bytes, |property| {
            match property {
                Property::SessionExpiryInterval(p) => session_expiry_interval = Some(p),
                Property::ReasonString(p) => reason_string = Some(p),
                Property::UserProperty(p) => user_properties.push(p),
                Property::ServerReference(p) => server_reference = Some(p),
                _ => {}, // Invalid property for packet
            }
        })?);
    }

    let packet = DisconnectPacket {
        reason,
        session_expiry_interval,
        reason_string,
        user_properties,
        server_reference,
    };

    Ok(Some(Packet::Disconnect(packet)))
}

fn decode_packet(
    packet_type: &PacketType,
    bytes: &mut Cursor<&mut BytesMut>,
    remaining_packet_length: u32,
) -> Result<Option<Packet>, DecodeError> {
    match packet_type {
        PacketType::Connect => decode_connect(bytes),
        PacketType::ConnectAck => decode_connect_ack(bytes),
        PacketType::Disconnect => decode_disconnect(bytes, remaining_packet_length),
        _ => Ok(None),
    }
}

pub fn decode_mqtt(bytes: &mut BytesMut) -> Result<Option<Packet>, DecodeError> {
    let mut bytes = Cursor::new(bytes);
    let first_byte = read_u8!(bytes);

    let first_byte_val = (first_byte & 0b1111_0000) >> 4;
    let packet_type = PacketType::try_from(first_byte_val)?;
    let remaining_packet_length = read_variable_int!(&mut bytes);

    let cursor_pos = bytes.position() as usize;
    let remaining_buffer_amount = bytes.get_ref().len() - cursor_pos;

    if remaining_buffer_amount < remaining_packet_length as usize {
        // If we don't have the full payload, just bail
        return Ok(None);
    }

    // TODO - use return_if_none! here after finishing decode_packet function
    let packet = decode_packet(&packet_type, &mut bytes, remaining_packet_length)?;

    let cursor_pos = bytes.position() as usize;
    let bytes = bytes.into_inner();

    let _rest = bytes.split_to(cursor_pos);

    Ok(packet)
}
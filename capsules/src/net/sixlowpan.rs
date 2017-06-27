/// Implements the 6LoWPAN specification for sending IPv6 datagrams over
/// 802.15.4 packets efficiently, as detailed in RFC 6282.

use net::ip::{IP6Header, IP6, MacAddr, IPAddr, IP6Proto};
use core::result::Result;

pub struct Context<'a> {
    prefix: &'a [u8],
    prefix_len: u8,
    id: u8,
    compress: bool,
}

pub trait ContextStore<'a> {
    fn get_context_from_addr(&self, ip_addr: IPAddr) -> Option<Context<'a>>;
    fn get_context_from_id(&self, ctx_id: u8) -> Option<Context<'a>>;
}

pub struct DummyStore {
}

impl<'a> ContextStore<'a> for DummyStore {
    fn get_context_from_addr(&self, ip_addr: IPAddr) -> Option<Context<'a>> {
        None
    }

    fn get_context_from_id(&self, ctx_id: u8) -> Option<Context<'a>> {
        None
    }
}

pub mod lowpan_iphc {
    use net::ip::MacAddr;
    use net::ip::MacAddr::*;

    pub const DISPATCH: [u8; 2]    = [0x60, 0x00];

    // First byte masks

    pub const TF_MASK: u8          = 0x18;
    pub const TF_TRAFFIC_CLASS: u8 = 0x08;
    pub const TF_FLOW_LABEL: u8    = 0x10;

    pub const NH: u8               = 0x04;

    pub const HLIM_MASK: u8        = 0x03;
    pub const HLIM_INLINE: u8      = 0x00;
    pub const HLIM_1: u8           = 0x01;
    pub const HLIM_64: u8          = 0x02;
    pub const HLIM_255: u8         = 0x03;

    // Second byte masks

    pub const CID: u8              = 0x80;

    pub const SAC: u8              = 0x40;

    pub const SAM_MASK: u8         = 0x30;
    pub const SAM_INLINE: u8       = 0x00;
    pub const SAM_64: u8           = 0x10;
    pub const SAM_16: u8           = 0x20;
    pub const SAM_0: u8            = 0x30;

    pub const MULTICAST: u8        = 0x01;

    pub const DAC: u8              = 0x04;
    pub const DAM_MASK: u8         = 0x03;
    pub const DAM_INLINE: u8       = 0x00;
    pub const DAM_64: u8           = 0x01;
    pub const DAM_16: u8           = 0x02;
    pub const DAM_0: u8            = 0x03;

    // Address compression
    pub const MAC_BASE: [u8; 8] = [0x00, 0x00, 0x00, 0xff, 0xfe, 0x00, 0x00, 0x00];
    pub const MAC_UL: u8 = 0x02;

    pub fn compute_iid(mac_addr: &MacAddr) -> [u8; 8] {
        match mac_addr {
            &ShortAddr(short_addr) => {
                // IID is 0000:00ff:fe00:XXXX, where XXXX is 16-bit MAC
                let mut iid: [u8; 8] = MAC_BASE;
                iid[6] = (short_addr >> 1) as u8;
                iid[7] = (short_addr & 0xff) as u8;
                iid
            },
            &LongAddr(long_addr) => {
                // IID is IEEE EUI-64 with universal/local bit inverted
                let mut iid: [u8; 8] = long_addr;
                long_addr[0] ^= MAC_UL;
                iid
            }
        }
    }
}

pub mod lowpan_nhc {
    pub const DISPATCH: u8 = 0xe0;

    pub const HOP_OPTS: u8 = 0 << 1;
    pub const ROUTING: u8  = 1 << 1;
    pub const FRAGMENT: u8 = 2 << 1;
    pub const DST_OPTS: u8 = 3 << 1;
    pub const MOBILITY: u8 = 4 << 1;
    pub const IP6: u8      = 7 << 1;
}

pub struct LoWPAN<'a, C: ContextStore<'a> + 'a> {
    ctx_store: &'a C,
}

impl<'a, C: ContextStore<'a> + 'a> LoWPAN<'a, C> {
    pub fn new(ctx_store: &'a C) -> LoWPAN<'a, C> {
        LoWPAN {
            ctx_store: ctx_store,
        }
    }

    /// Constructs a 6LoWPAN header in `buf` from the given IPv6 header and
    /// 16-bit MAC addresses.  Returns the number of bytes written into `buf`.
    pub fn compress(&self,
                    ip6_header: &IP6Header,
                    src_mac_addr: MacAddr,
                    dest_mac_addr: MacAddr,
                    buf: &'static mut [u8]) -> usize {
        // The first two bytes are the LOWPAN_IPHC header
        let mut offset: usize = 2;

        // Initialize the LOWPAN_IPHC header
        buf[0..2].copy_from_slice(&lowpan_iphc::DISPATCH);

        let mut src_ctx: Option<Context> = self.ctx_store.get_context_from_addr(ip6_header.src_addr);
        let mut dst_ctx: Option<Context> = self.ctx_store.get_context_from_addr(ip6_header.dst_addr);

        // Do not use these contexts if they are not to be used for compression
        src_ctx = src_ctx.and_then(|ctx| { if ctx.compress { Some(ctx) } else { None } });
        dst_ctx = dst_ctx.and_then(|ctx| { if ctx.compress { Some(ctx) } else { None } });

        // Context Identifier Extension
        self.compress_cie(&src_ctx, &dst_ctx, buf, &mut offset);

        // Traffic Class & Flow Label
        self.compress_tf(ip6_header, buf, &mut offset);

        // Next Header
        self.compress_nh(ip6_header, buf, &mut offset);

        // Hop Limit
        self.compress_hl(ip6_header, buf, &mut offset);

        // Source Address
        self.compress_src(&ip6_header.src_addr, &src_mac_addr, &src_ctx, buf, &mut offset);

        offset
    }

    fn compress_cie(&self,
                    src_ctx: &Option<Context>,
                    dst_ctx: &Option<Context>,
                    buf: &'static mut [u8],
                    offset: &mut usize) {
        let mut cie: u8 = 0;

        src_ctx.map(|ctx| {
            if ctx.id != 0 { cie |= ctx.id << 4; }
        });
        dst_ctx.map(|ctx| {
            if ctx.id != 0 { cie |= ctx.id; }
        });

        if cie != 0 {
            buf[1] |= lowpan_iphc::CID;
            buf[*offset] = cie;
            *offset += 1;
        }
    }

    fn compress_tf(&self,
                   ip6_header: &IP6Header,
                   buf: &'static mut [u8],
                   offset: &mut usize) {
        // TODO: All of this needs to be checked for endian-ness and correctness
        // TODO: Remove version?
        let version = ip6_header.version_class_flow[0] >> 4;
        let class   = ((ip6_header.version_class_flow[0] << 4) & 0xf)
                    | ((ip6_header.version_class_flow[1] >> 4) & 0x0f);
        let ecn     = (class >> 6) & 0b11000000; // Gets leading 2 bits
        let dscp    = class & 0b00111111;  // Gets trailing 6 bits
        let mut flow: [u8; 3];
        flow[0] = ip6_header.version_class_flow[1] & 0xf; // Zero upper 4 bits
        flow[1] = ip6_header.version_class_flow[2];
        flow[2] = ip6_header.version_class_flow[3];

        let mut tf_encoding = 0;

        // Flow label is all zeroes and can be elided
        if flow[0] == 0 && flow[1] == 0 && flow[2] == 0 {
            // The 1X cases
            tf_encoding |= lowpan_iphc::TF_FLOW_LABEL;
        }

        // DSCP can be elided, but ECN elided only if flow also elided
        // X1 cases
        if dscp == 0 {
            // If flow *not* elided, combine with ECN
            // 01 case
            if tf_encoding == 0 {
                buf[*offset] = (ecn << 6 & 0b11000000) | flow[0];
                buf[*offset + 1] = flow[1];
                buf[*offset + 2] = flow[2];
                *offset += 3;
            }
            tf_encoding |= lowpan_iphc::TF_TRAFFIC_CLASS;
        // X0 cases
        } else {
            // If DSCP cannot be elided
            buf[*offset] = class;
            *offset += 1;

            // 00 case
            if tf_encoding == 0 {
                buf[*offset] = flow[0] & 0xf;
                buf[*offset + 1] = flow[1];
                buf[*offset + 2] = flow[2];
                *offset += 3;
            }
        }
        buf[0] |= tf_encoding;
    }

    fn ip6_proto_to_nhc_eid(next_header: u8) -> Option<u8> {
        match next_header {
            IP6Proto::HOP_OPTS => Some(lowpan_nhc::HOP_OPTS),
            IP6Proto::ROUTING  => Some(lowpan_nhc::ROUTING),
            IP6Proto::FRAGMENT => Some(lowpan_nhc::FRAGMENT),
            IP6Proto::DST_OPTS => Some(lowpan_nhc::DST_OPTS),
            IP6Proto::MOBILITY => Some(lowpan_nhc::MOBILITY),
            IP6Proto::IP6      => Some(lowpan_nhc::IP6),
            _ => None,
        }
    }

    fn compress_nh(&self,
                   ip6_header: &IP6Header,
                   buf: &'static mut [u8],
                   offset: &mut usize) {
        if LoWPAN::ip6_proto_to_nhc_eid(ip6_header.next_header).is_some() {
            buf[0] |= lowpan_iphc::NH;
        } else {
            buf[*offset] = ip6_header.next_header;
            *offset += 1;
        }
    }

    fn compress_hl(&self,
                   ip6_header: &IP6Header,
                   buf: &'static mut [u8],
                   offset: &mut usize) {
        let hop_limit_flag = {
            match ip6_header.hop_limit {
                // Compressed
                1   => lowpan_iphc::HLIM_1,
                64  => lowpan_iphc::HLIM_64, 
                255 => lowpan_iphc::HLIM_255,
                // Uncompressed
                _   => {
                    buf[*offset] = ip6_header.hop_limit;
                    *offset += 1;
                    lowpan_iphc::HLIM_INLINE
                },
            }
        };
        buf[0] |= hop_limit_flag;
    }

    fn compress_src(&self,
                    src_ip_addr: &IPAddr,
                    src_mac_addr: &MacAddr,
                    src_ctx: &Option<Context>,
                    buf: &'static mut [u8],
                    offset: &mut usize) {
        if IP6::addr_is_unspecified(src_ip_addr) {
            // SAC = 1, SAM = 00
            buf[1] |= lowpan_iphc::SAC;
        } else if IP6::addr_is_link_local(src_ip_addr) {
            // SAC = 0, SAM = 01, 10, 11
            self.compress_src_iid(src_ip_addr, src_mac_addr, src_ctx, buf, offset);
        } else if !src_ctx.is_none() {
            // SAC = 1, SAM = 01, 10, 11
            buf[1] |= lowpan_iphc::SAC;
            self.compress_src_iid(src_ip_addr, src_mac_addr, src_ctx, buf, offset);
        } else {
            // SAC = 0, SAM = 00
            buf[*offset..*offset + 16].copy_from_slice(src_ip_addr);
            *offset += 16;
        }
    }

    fn compress_src_iid(&self,
                        src_ip_addr: &IPAddr,
                        src_mac_addr: &MacAddr,
                        src_ctx: &Option<Context>,
                        buf: &'static mut [u8],
                        offset: &mut usize) {
        let iid: [u8; 8] = lowpan_iphc::compute_iid(src_mac_addr);
        if src_ip_addr[8..16] == iid {
            // SAM = 11
            buf[1] |= lowpan_iphc::SAM_0;
        } else if src_ip_addr[8..14] == lowpan_iphc::MAC_BASE[0..6] {
            // SAM = 10
            buf[1] |= lowpan_iphc::SAM_16;
            buf[*offset..*offset + 2].copy_from_slice(&src_ip_addr[14..16]);
            *offset += 2;
        } else {
            // SAM = 01
            buf[1] |= lowpan_iphc::SAM_64;
            buf[*offset..*offset + 8].copy_from_slice(&src_ip_addr[8..16]);
            *offset += 8;
        }
    }

    /// Decodes the compressed header into a full IPv6 header given the 16-bit
    /// MAC addresses. `buf` is expected to be a slice starting from the
    /// beginning of the IP header.  Returns the number of bytes taken up by the
    /// header, so the remaining bytes are the payload. Also returns an optional
    /// `FragInfo` containing the datagram tag and fragmentation offset if this
    /// packet is part of a set of fragments.
    pub fn decompress(&self,
                      buf: &'static mut [u8],
                      src_mac_addr: MacAddr,
                      dest_mac_addr: MacAddr,
                      mesh_local_prefix: &[u8])
                      -> Result<(IP6Header, usize, Option<FragInfo>), ()> {
    }
}

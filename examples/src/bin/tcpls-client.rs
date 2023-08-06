
/// Simplified version of tlsclient-mio
/// to show tcpls features

use std::process;
use std::sync::Arc;
use log::debug;
use mio::net::TcpStream;
//use rustls::ClientConfig;
use rustls::tcpls::Role;
use rustls::tcpls::TcplsConnection;
use rustls::tcpls::stream::TcplsStreamBuilder;
use rustls::tcpls::utils::constant;
use std::fs::File;

use std::fs;
use std::io;
use std::io::{BufReader, Read, Write};
use std::net::SocketAddr;
use std::str;
use chrono;

#[macro_use]
extern crate serde_derive;

use docopt::Docopt;

use rustls::{OwnedTrustAnchor, RootCertStore};

const CLIENT: mio::Token = mio::Token(0);

#[cfg(not(feature = "demo"))]
macro_rules! demo_println {
    ($($arg:tt)*) => {};
}

#[cfg(feature = "demo")]
macro_rules! demo_println {
    ($($arg:tt)*) => {
        let timestamp = chrono::Local::now();
        println!("[{}] {}", timestamp.format("%H:%M:%S%.6f"), format_args!($($arg)*));
    };
}

#[derive(PartialEq)]
enum Mode {
    Default,
    Ping,
    Streams,
}

/// This encapsulates the TCP-level connection, some connection
/// state, and the underlying TLS-level session.
struct TcplsClient {
    socket: TcpStream,
    closing: bool,
    clean_closure: bool,
    tls_conn: rustls::ClientConnection,
    //tls_cfg: Arc<ClientConfig>,
    tcpls: TcplsConnection,
    mode: Mode,
}

impl TcplsClient {
    fn new(
        sock: TcpStream,
        server_name: rustls::ServerName,
        cfg: Arc<rustls::ClientConfig>,
    ) -> Self {
        Self {
            socket: sock,
            closing: false,
            clean_closure: false,
            //tls_cfg: cfg.clone(),
            tls_conn: rustls::ClientConnection::new(cfg, server_name).unwrap(),
            tcpls: TcplsConnection::new(0, Role::Client),
            mode: Mode::Default,
        }
    }

    /// change mode of the client
    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
    }

    /// Handles events sent to the TlsClient by mio::Poll
    fn ready(&mut self, ev: &mio::event::Event) {
        assert_eq!(ev.token(), CLIENT);
        if ev.is_readable() {
            self.do_read();
        }

        if ev.is_writable() && self.tls_conn.is_handshaking() {
            debug!("Handshake ongoing!");
            // send padding frame while making the hanshake to avoid
            // blocking of connection.
            self.tls_conn.writer().write_all(&[constant::PADDING_FRAME]).unwrap();
            self.do_write();
        }

        if ev.is_writable() && !self.tls_conn.is_handshaking() {
            self.do_write();
        }

        if self.is_closed() {
            println!("Connection closed");
            process::exit(if self.clean_closure { 0 } else { 1 });
        }
    }

    fn read_source_to_end(&mut self, rd: &mut dyn io::Read) -> io::Result<usize> {
        let mut buf = Vec::new();
        let len = rd.read_to_end(&mut buf)?;
        self.tcpls.set_data(&buf);
        demo_println!("sending {}", std::str::from_utf8(&buf).expect("failed to read input data"));
        self.tls_conn
            .writer()
            .write_all(&self.tcpls.create_record())
            .unwrap();

        Ok(len)
    }

    fn send_data(&mut self) {
        if self.tcpls.has_data() {
            let data = self.tcpls.process_w().unwrap();
            debug!("data len: {}", data.len());
            debug!("sending data");
            demo_println!("Sending record of len {}", data.len());
            println!("last frame of the record: {}",data[data.len()-1]);
            self.tls_conn
            .writer()
            .write(&data).unwrap();
        }
    }

    /// We're ready to do a read.
    fn do_read(&mut self) {
        // Read TLS data.  This fails if the underlying TCP connection
        // is broken.
        match self.tls_conn.read_tls(&mut self.socket) {
            Err(error) => {
                if error.kind() == io::ErrorKind::WouldBlock {
                    return;
                }
                println!("TLS read error: {:?}", error);
                self.closing = true;
                return;
            }

            // If we're ready but there's no data: EOF.
            Ok(0) => {
                println!("EOF");
                self.closing = true;
                self.clean_closure = true;
                return;
            }

            Ok(_) => {}
        };

        // Reading some TLS data might have yielded new TLS
        // messages to process.  Errors from this indicate
        // TLS protocol problems and are fatal.
        let io_state = match self.tls_conn.process_new_packets() {
            Ok(io_state) => io_state,
            Err(err) => {
                println!("TLS error: {:?}", err);
                self.closing = true;
                return;
            }
        };

        // Having read some TLS data, and processed any new messages,
        // we might have new plaintext as a result.
        //
        // Read it and then write it to stdout.
        if io_state.plaintext_bytes_to_read() > 0 {
            let mut plaintext = Vec::new();
            plaintext.resize(io_state.plaintext_bytes_to_read(), 0u8);
            self.tls_conn
                .reader()
                .read_exact(&mut plaintext)
                .unwrap();
            
            let _ = self.tcpls.process_r(&plaintext);
            //println!("{}\n", self.tcpls_conn.get_stream_data());
            match self.mode {
                Mode::Default => {
                    let buf = self.tcpls.get_stream_data(0).expect("Failed to read TCPLS Stream");
                    demo_println!("received: {}", std::str::from_utf8(&buf).expect("Failed to read utf-8 sequence"));
                },
                Mode::Streams | Mode::Ping => {
                    demo_println!("Ack received\n{}", self.tcpls.get_last_ack_info());
                    
                },
            }
        }

        // If wethat fails, the peer might have started a clean TLS-level
        // session closure.
        if io_state.peer_has_closed() {
            self.clean_closure = true;
            self.closing = true;
        }
    }

    /// print the data received by a stream with the given id
    fn _print_tcpls_stream(&mut self, id: u32) {
        let stream_data = match self.tcpls.get_stream_data(id) {
            Ok(data) => data,
            Err(err) => {
                println!("TCPLS Error while reading data: {:?}", err);
                return;
            }
        };
        demo_println!("{}", std::str::from_utf8(stream_data).unwrap());
        
        io::stdout()
            .write_all(stream_data)
            .unwrap();

        println!("{}", std::str::from_utf8(stream_data).unwrap());
    }

    fn do_write(&mut self) {
        self.tls_conn
            .write_tls(&mut self.socket)
            .unwrap();
    }

    /// Registers self as a 'listener' in mio::Registry
    fn register(&mut self, registry: &mio::Registry) {
        let interest = self.event_set();
        registry
            .register(&mut self.socket, CLIENT, interest)
            .unwrap();
    }

    /// Reregisters self as a 'listener' in mio::Registry.
    fn reregister(&mut self, registry: &mio::Registry) {
        let interest = self.event_set();
        registry
            .reregister(&mut self.socket, CLIENT, interest)
            .unwrap();
    }

    /// Use wants_read/wants_write to register for different mio-level
    /// IO readiness events.
    fn event_set(&self) -> mio::Interest {
        let rd = self.tls_conn.wants_read();
        let wr = self.tls_conn.wants_write();

        if rd && wr {
            mio::Interest::READABLE | mio::Interest::WRITABLE
        } else if wr {
            mio::Interest::WRITABLE
        } else {
            mio::Interest::READABLE
        }
    }

    fn is_closed(&self) -> bool {
        self.closing
    }
}
impl io::Write for TcplsClient {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.tls_conn.writer().write(bytes)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.tls_conn.writer().flush()
    }
}

impl io::Read for TcplsClient {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        self.tls_conn.reader().read(bytes)
    }
}

const USAGE: &str = "
Connects to the TCPLS server at hostname:PORT.  The default PORT
is 443.  3 features are proposed default one is read the stdin until
EOF, --ping send every interval a Ping frame and receive 
an Ack frame.  --file transfer a file to the server /.

If --cafile is not supplied, a built-in set of CA certificates
are used from the webpki-roots crate.

Usage:
  tcpls-client [options] [--suite SUITE ...] [--proto PROTO ...] [--protover PROTOVER ...] <hostname>
  tcpls-client (--version | -v)
  tcpls-client (--help | -h)

Options:
    -p, --port PORT     Connect to PORT [default: 443].
    --file FILES        Send FILES to the server (if multiple files given. put them quoted and separated by a space).
    --ping              ping the server and wait for an ack 10 times.
    --cafile CAFILE     Read root certificates from CAFILE.
    --auth-key KEY      Read client authentication key from KEY.
    --auth-certs CERTS  Read client authentication certificates from CERTS.
                        CERTS must match up with KEY.
    --protover VERSION  Disable default TLS version list, and use
                        VERSION instead.  May be used multiple times.
    --suite SUITE       Disable default cipher suite list, and use
                        SUITE instead.  May be used multiple times.
    --proto PROTOCOL    Send ALPN extension containing PROTOCOL.
                        May be used multiple times to offer several protocols.
    --no-tickets        Disable session ticket support.
    --no-sni            Disable server name indication support.
    --insecure          Disable certificate verification.
    --verbose           Emit log output.
    --max-frag-size M   Limit outgoing messages to M bytes.
    --version, -v       Show tool version.
    --help, -h          Show this screen.
";

#[derive(Debug, Deserialize)]
struct Args {
    flag_port: Option<u16>,
    flag_file: Option<String>,
    flag_ping: bool,
    flag_verbose: bool,
    flag_protover: Vec<String>,
    flag_suite: Vec<String>,
    flag_proto: Vec<String>,
    flag_max_frag_size: Option<usize>,
    flag_cafile: Option<String>,
    flag_no_tickets: bool,
    flag_no_sni: bool,
    flag_insecure: bool,
    flag_auth_key: Option<String>,
    flag_auth_certs: Option<String>,
    arg_hostname: String,
}

// TODO: um, well, it turns out that openssl s_client/s_server
// that we use for testing doesn't do ipv6.  So we can't actually
// test ipv6 and hence kill this.
fn lookup_ipv4(host: &str, port: u16) -> SocketAddr {
    use std::net::ToSocketAddrs;

    let addrs = (host, port).to_socket_addrs().unwrap();
    for addr in addrs {
        if let SocketAddr::V4(_) = addr {
            return addr;
        }
    }

    unreachable!("Cannot lookup address");
}

/// Find a ciphersuite with the given name
fn find_suite(name: &str) -> Option<rustls::SupportedCipherSuite> {
    for suite in rustls::ALL_CIPHER_SUITES {
        let sname = format!("{:?}", suite.suite()).to_lowercase();

        if sname == name.to_string().to_lowercase() {
            return Some(*suite);
        }
    }

    None
}

/// Make a vector of ciphersuites named in `suites`
fn lookup_suites(suites: &[String]) -> Vec<rustls::SupportedCipherSuite> {
    let mut out = Vec::new();

    for csname in suites {
        let scs = find_suite(csname);
        match scs {
            Some(s) => out.push(s),
            None => panic!("cannot look up ciphersuite '{}'", csname),
        }
    }

    out
}

/// Make a vector of protocol versions named in `versions`
fn lookup_versions(versions: &[String]) -> Vec<&'static rustls::SupportedProtocolVersion> {
    let mut out = Vec::new();

    for vname in versions {
        let version = match vname.as_ref() {
            "1.2" => &rustls::version::TLS12,
            "1.3" => &rustls::version::TLS13,
            _ => panic!(
                "cannot look up version '{}', valid are '1.2' and '1.3'",
                vname
            ),
        };
        out.push(version);
    }

    out
}

fn load_certs(filename: &str) -> Vec<rustls::Certificate> {
    let certfile = fs::File::open(filename).expect("cannot open certificate file");
    let mut reader = BufReader::new(certfile);
    rustls_pemfile::certs(&mut reader)
        .unwrap()
        .iter()
        .map(|v| rustls::Certificate(v.clone()))
        .collect()
}

fn load_private_key(filename: &str) -> rustls::PrivateKey {
    let keyfile = fs::File::open(filename).expect("cannot open private key file");
    let mut reader = BufReader::new(keyfile);

    loop {
        match rustls_pemfile::read_one(&mut reader).expect("cannot parse private key .pem file") {
            Some(rustls_pemfile::Item::RSAKey(key)) => return rustls::PrivateKey(key),
            Some(rustls_pemfile::Item::PKCS8Key(key)) => return rustls::PrivateKey(key),
            Some(rustls_pemfile::Item::ECKey(key)) => return rustls::PrivateKey(key),
            None => break,
            _ => {}
        }
    }

    panic!(
        "no keys found in {:?} (encrypted keys not supported)",
        filename
    );
}

#[cfg(feature = "dangerous_configuration")]
mod danger {
    pub struct NoCertificateVerification {}

    impl rustls::client::ServerCertVerifier for NoCertificateVerification {
        fn verify_server_cert(
            &self,
            _end_entity: &rustls::Certificate,
            _intermediates: &[rustls::Certificate],
            _server_name: &rustls::ServerName,
            _scts: &mut dyn Iterator<Item = &[u8]>,
            _ocsp: &[u8],
            _now: std::time::SystemTime,
        ) -> Result<rustls::client::ServerCertVerified, rustls::Error> {
            Ok(rustls::client::ServerCertVerified::assertion())
        }
    }
}

#[cfg(feature = "dangerous_configuration")]
fn apply_dangerous_options(args: &Args, cfg: &mut rustls::ClientConfig) {
    if args.flag_insecure {
        cfg.dangerous()
            .set_certificate_verifier(Arc::new(danger::NoCertificateVerification {}));
    }
}

#[cfg(not(feature = "dangerous_configuration"))]
fn apply_dangerous_options(args: &Args, _: &mut rustls::ClientConfig) {
    if args.flag_insecure {
        panic!("This build does not support --insecure.");
    }
}

/// Build a `ClientConfig` from our arguments
fn make_config(args: &Args) -> Arc<rustls::ClientConfig> {
    let mut root_store = RootCertStore::empty();

    if args.flag_cafile.is_some() {
        let cafile = args.flag_cafile.as_ref().unwrap();

        let certfile = fs::File::open(cafile).expect("Cannot open CA file");
        let mut reader = BufReader::new(certfile);
        root_store.add_parsable_certificates(&rustls_pemfile::certs(&mut reader).unwrap());
    } else {
        root_store.add_server_trust_anchors(
            webpki_roots::TLS_SERVER_ROOTS
                .0
                .iter()
                .map(|ta| {
                    OwnedTrustAnchor::from_subject_spki_name_constraints(
                        ta.subject,
                        ta.spki,
                        ta.name_constraints,
                    )
                }),
        );
    }

    let suites = if !args.flag_suite.is_empty() {
        lookup_suites(&args.flag_suite)
    } else {
        rustls::DEFAULT_CIPHER_SUITES.to_vec()
    };

    let versions = if !args.flag_protover.is_empty() {
        lookup_versions(&args.flag_protover)
    } else {
        rustls::DEFAULT_VERSIONS.to_vec()
    };

    let config = rustls::ClientConfig::builder()
        .with_cipher_suites(&suites)
        .with_safe_default_kx_groups()
        .with_protocol_versions(&versions)
        .expect("inconsistent cipher-suite/versions selected")
        .with_root_certificates(root_store);

    let mut config = match (&args.flag_auth_key, &args.flag_auth_certs) {
        (Some(key_file), Some(certs_file)) => {
            let certs = load_certs(certs_file);
            let key = load_private_key(key_file);
            config
                .with_single_cert(certs, key)
                .expect("invalid client auth certs/key")
        }
        (None, None) => config.with_no_client_auth(),
        (_, _) => {
            panic!("must provide --auth-certs and --auth-key together");
        }
    };

    config.key_log = Arc::new(rustls::KeyLogFile::new());

    if args.flag_no_tickets {
        config.enable_tickets = false;
    }

    if args.flag_no_sni {
        config.enable_sni = false;
    }

    config.tcpls_enabled = true;


    config.alpn_protocols = args
        .flag_proto
        .iter()
        .map(|proto| proto.as_bytes().to_vec())
        .collect();
    config.max_fragment_size = args.flag_max_frag_size;

    apply_dangerous_options(args, &mut config);
    config.enable_early_data = false;

    Arc::new(config)
}

/// ping continuously the server running tcpls and wait for an ack
fn ping_server(tlsclient: &mut TcplsClient) {
    // send a ping and wait an Ack over a TCPLS connection
    let mut index = 0;
    tlsclient.set_mode(Mode::Ping);
    let ping: [u8;1] = [constant::PING_FRAME];
    let padding: [u8; 1] = [constant::PADDING_FRAME];

    let mut poll = mio::Poll::new().unwrap();
    let mut events = mio::Events::with_capacity(32);
    tlsclient.register(poll.registry());
    tlsclient.tcpls.inv_ack();
    while index < 10 {
        poll.poll(&mut events, None).unwrap();

        for ev in events.iter() {
            tlsclient.ready(ev);
            tlsclient.reregister(poll.registry());
        }
    
        if !tlsclient.tls_conn.is_handshaking() && tlsclient.tcpls.has_received_ack() {
            debug!("sending a ping {:?}", &ping);
            demo_println!("Sending a Ping");
            tlsclient.tls_conn
                .writer()
                .write_all(&ping)
                .unwrap();
        
            tlsclient.tcpls.inv_ack(); // wait for next ack before resending a ping
            index += 1;
        }
        tlsclient.tcpls.update_tls_seq(tlsclient.tls_conn.get_tls_record_seq());

        tlsclient.tls_conn
                .writer()
                .write_all(&padding)
                .unwrap(); // To avoid a double ping 

    }

    demo_println!("10 ping send");
    process::exit(if tlsclient.clean_closure { 0 } else { 1 });
    
}

/// Parse some arguments, then make a TLS client connection
/// somewhere.
fn main() {
    demo_println!("Demo mode enabled");
    let version = env!("CARGO_PKG_NAME").to_string() + ", version: " + env!("CARGO_PKG_VERSION");

    let args: Args = Docopt::new(USAGE)
        .map(|d| d.help(true))
        .map(|d| d.version(Some(version)))
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());

    if args.flag_verbose {
        env_logger::Builder::new()
            .parse_filters("trace")
            .init();
    }

    let port = args.flag_port.unwrap_or(443);
    let addr = lookup_ipv4(args.arg_hostname.as_str(), port);

    let config = make_config(&args);

    let sock = TcpStream::connect(addr).unwrap();
    let server_name = args
        .arg_hostname
        .as_str()
        .try_into()
        .expect("invalid DNS name");

    let mut tlsclient = TcplsClient::new(sock, server_name, config);

    // Where serious things begin

    if args.flag_file.is_some() {
        // send a file over a TCPLS connection
        tlsclient.set_mode(Mode::Streams);
        demo_println!("Feature streams enabled");
        let mut stream_id = 0;
        let raw_filenames = match args.flag_file {
            Some(x) => x,
            None => panic!("No file given!"),
        };
        
        let filenames: Vec<&str> = raw_filenames.split(' ').collect();
        for filename in filenames {
            demo_println!("giving stream {stream_id} to file: {filename}");
            let mut file = File::open(filename).expect("Error while opening file");
            let mut data: Vec<u8> = Vec::new();
            let _ = file.read_to_end(&mut data).expect("error while reading file");
            let mut n_stream = TcplsStreamBuilder::new(stream_id);
            n_stream.add_data(&data);
            tlsclient.tcpls.add_stream(n_stream.build(), stream_id);
            stream_id += 2;
        }
        debug!("streams created");
        tlsclient.tcpls.dbg_streams();

        // send file

        let mut poll = mio::Poll::new().unwrap();
        let mut events = mio::Events::with_capacity(32);

        tlsclient.register(poll.registry());
        debug!("Sending file");
        loop {
            poll.poll(&mut events, None).unwrap();
            if !tlsclient.tls_conn.is_handshaking() {
                tlsclient.send_data();
            } else {
                tlsclient.tls_conn.writer().write_all(&[constant::PING_FRAME])
                                        .expect("error while sending ping");
            }

            for ev in events.iter() {
                tlsclient.ready(ev);
                tlsclient.reregister(poll.registry());
            }
            tlsclient.tcpls.update_tls_seq(tlsclient.tls_conn.get_tls_record_seq());
            
            if !tlsclient.tcpls.has_data() {
                demo_println!("All data send");
                demo_println!("{}", tlsclient.tcpls.get_streams_sent_info());
                // process::exit(if tlsclient.clean_closure { 0 } else { 1 });
            }
        }

    } else if args.flag_ping {
        demo_println!("Feature Ping enabled");
        ping_server(&mut tlsclient);
    } else {
        demo_println!("Default feature enabled");
        // send a string over a TCPLS connection
        let mut stdin = io::stdin();

        tlsclient.read_source_to_end(&mut stdin).unwrap();

        let mut poll = mio::Poll::new().unwrap();
        let mut events = mio::Events::with_capacity(32);
        tlsclient.register(poll.registry());
        loop {
            poll.poll(&mut events, None).unwrap();

            for ev in events.iter() {
                tlsclient.ready(ev);
                tlsclient.reregister(poll.registry());
            }
            tlsclient.tcpls.update_tls_seq(tlsclient.tls_conn.get_tls_record_seq());
        }
    }
}


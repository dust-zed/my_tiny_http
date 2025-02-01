#[cfg(feature = "ssl-openssl")]
pub(crate) mod openssl;
#[cfg(feature = "ssl-openssl")]
pub(crate) use self::openssl::OpenSslContext as SslContextImpl;
#[cfg(feature = "ssl-openssl")]
pub(crate) use self::openssl::SplitOpenSslStream as SslStream;

#[cfg(feature = "ssl-rustls")]
pub(crate) mod rustls;
#[cfg(feature = "ssl-rustls")]
pub(crate) use self::rustls::RustlsContext as SslContextImpl;
#[cfg(feature = "ssl-rustls")]
pub(crate) use self::rustls::RustlsStream as SslStream;

#[cfg(feature = "ssl-native-tls")]
pub(crate) mod native_tls;
#[cfg(feature = "ssl-native-tls")]
pub(crate) use self::native_tls::NativeTlsContext as SslContextImpl;
#[cfg(feature = "ssl-native-tls")]
pub(crate) use self::native_tls::NativeTlsStream as SslStream;
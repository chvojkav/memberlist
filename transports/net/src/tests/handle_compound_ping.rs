use memberlist_core::transport::{tests::handle_compound_ping, Lpe};
use nodecraft::{resolver::socket_addr::SocketAddrResolver, CheapClone};

use crate::{NetTransport, NetTransportOptions};

use super::*;

#[cfg(all(feature = "compression", feature = "encryption"))]
pub async fn compound_ping<S, R>(s: S, kind: AddressKind) -> Result<(), AnyError>
where
  S: StreamLayer,
  R: Runtime,
{
  let name = format!("{kind}_compound_ping");
  let label = Label::try_from(&name)?;
  let pk = SecretKey::from([1; 32]);
  let client = NetTransportTestClient::<R>::new(kind.next(0))
    .await?
    .with_label(label.cheap_clone())
    .with_send_label(true)
    .with_receive_encrypted(Some(pk))
    .with_receive_compressed(true)
    .with_receive_verify_label(true);

  let mut opts = NetTransportOptions::new(name.into())
    .with_primary_key(Some(pk))
    .with_encryption_algo(Some(EncryptionAlgo::PKCS7))
    .with_gossip_verify_outgoing(true)
    .with_compressor(Some(Compressor::default()))
    .with_label(label)
    .with_offload_size(20);
  opts.add_bind_address(kind.next(0));
  let trans = NetTransport::<_, _, _, Lpe<_, _>, _>::new(SocketAddrResolver::<R>::new(), s, opts)
    .await
    .unwrap();
  handle_compound_ping(trans, client, super::compound_encoder).await?;
  Ok(())
}

#[cfg(feature = "compression")]
pub async fn compound_ping_compression_only<S, R>(s: S, kind: AddressKind) -> Result<(), AnyError>
where
  S: StreamLayer,
  R: Runtime,
{
  let name = format!("{kind}_compound_ping");
  let label = Label::try_from(&name)?;
  let client = NetTransportTestClient::<R>::new(kind.next(0))
    .await?
    .with_label(label.cheap_clone())
    .with_send_label(true)
    .with_receive_compressed(true)
    .with_receive_verify_label(true);

  let mut opts = NetTransportOptions::new(name.into())
    .with_compressor(Some(Compressor::default()))
    .with_label(label)
    .with_offload_size(10);
  opts.add_bind_address(kind.next(0));
  let trans = NetTransport::<_, _, _, Lpe<_, _>, _>::new(SocketAddrResolver::<R>::new(), s, opts)
    .await
    .unwrap();
  handle_compound_ping(trans, client, super::compound_encoder).await?;
  Ok(())
}

#[cfg(feature = "encryption")]
pub async fn compound_ping_encryption_only<S, R>(s: S, kind: AddressKind) -> Result<(), AnyError>
where
  S: StreamLayer,
  R: Runtime,
{
  let name = format!("{kind}_compound_ping");
  let label = Label::try_from(&name)?;
  let pk = SecretKey::from([1; 32]);
  let client = NetTransportTestClient::<R>::new(kind.next(0))
    .await?
    .with_label(label.cheap_clone())
    .with_send_label(true)
    .with_receive_encrypted(Some(pk))
    .with_receive_verify_label(true);

  let mut opts = NetTransportOptions::new(name.into())
    .with_primary_key(Some(pk))
    .with_encryption_algo(Some(EncryptionAlgo::PKCS7))
    .with_label(label)
    .with_offload_size(10);
  opts.add_bind_address(kind.next(0));
  let trans = NetTransport::<_, _, _, Lpe<_, _>, _>::new(SocketAddrResolver::<R>::new(), s, opts)
    .await
    .unwrap();
  handle_compound_ping(trans, client, super::compound_encoder).await?;
  Ok(())
}

#[cfg(not(any(feature = "compression", feature = "encryption")))]
pub async fn compound_ping_no_encryption_no_compression<S, R>(
  s: S,
  kind: AddressKind,
) -> Result<(), AnyError>
where
  S: StreamLayer,
  R: Runtime,
{
  let name = format!("{kind}_compound_ping");
  let label = Label::try_from(&name)?;
  let client = NetTransportTestClient::<R>::new(kind.next(0))
    .await?
    .with_label(label.cheap_clone())
    .with_send_label(true)
    .with_receive_verify_label(true);

  let mut opts = NetTransportOptions::new(name.into()).with_label(label);
  opts.add_bind_address(kind.next(0));
  let trans = NetTransport::<_, _, _, Lpe<_, _>, _>::new(SocketAddrResolver::<R>::new(), s, opts)
    .await
    .unwrap();
  handle_compound_ping(trans, client, super::compound_encoder).await?;
  Ok(())
}

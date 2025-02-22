use crate::crypto::{
    helper, Blake2b256, CryptoRngCore, Deserialize, Digest, DigestSigner, KeyPair, NistP256,
    Random, Secp256k1, Serialize,
};
use generic_array::typenum::Unsigned;
use std::convert::Infallible;

#[derive(Debug, Clone)]
pub struct Signature<C>(C);

impl<C> core::ops::Deref for Signature<C> {
    type Target = C;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Serialize for Signature<ecdsa::Signature<Secp256k1>> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(&self.0.to_bytes())
    }
}

impl Serialize for Signature<ecdsa::Signature<NistP256>> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(&self.0.to_bytes())
    }
}

impl<'de> Deserialize<'de> for Signature<ecdsa::Signature<Secp256k1>> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes = deserializer.deserialize_bytes(helper::ByteArrayVisitor::<
            { ecdsa::SignatureSize::<Secp256k1>::USIZE },
        >::new())?;
        match ecdsa::Signature::from_bytes(&bytes.into()) {
            Ok(val) => Ok(Self(val)),
            Err(err) => Err(serde::de::Error::custom(err)),
        }
    }
}

impl<'de> Deserialize<'de> for Signature<ecdsa::Signature<NistP256>> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes = deserializer.deserialize_bytes(helper::ByteArrayVisitor::<
            { ecdsa::SignatureSize::<NistP256>::USIZE },
        >::new())?;
        match ecdsa::Signature::from_bytes(&bytes.into()) {
            Ok(val) => Ok(Self(val)),
            Err(err) => Err(serde::de::Error::custom(err)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SigningKey<C>(pub(crate) C);

impl Random for SigningKey<ecdsa::SigningKey<Secp256k1>> {
    type Error = Infallible;
    fn random<R: CryptoRngCore>(r: &mut R) -> Result<Self, Self::Error> {
        Ok(SigningKey(ecdsa::SigningKey::random(r)))
    }
}

impl Random for SigningKey<ecdsa::SigningKey<NistP256>> {
    type Error = Infallible;
    fn random<R: CryptoRngCore>(r: &mut R) -> Result<Self, Self::Error> {
        Ok(SigningKey(ecdsa::SigningKey::random(r)))
    }
}

impl<C> core::ops::Deref for SigningKey<C> {
    type Target = C;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Serialize for SigningKey<ecdsa::SigningKey<Secp256k1>> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(&self.0.to_bytes())
    }
}

impl Serialize for SigningKey<ecdsa::SigningKey<NistP256>> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(&self.0.to_bytes())
    }
}

impl<'de> Deserialize<'de> for SigningKey<ecdsa::SigningKey<Secp256k1>> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes = deserializer.deserialize_bytes(helper::ByteArrayVisitor::<
            { elliptic_curve::FieldBytesSize::<Secp256k1>::USIZE },
        >::new())?;
        match ecdsa::SigningKey::from_bytes(&bytes.into()) {
            Ok(val) => Ok(Self(val)),
            Err(err) => Err(serde::de::Error::custom(err)),
        }
    }
}

impl<'de> Deserialize<'de> for SigningKey<ecdsa::SigningKey<NistP256>> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes = deserializer.deserialize_bytes(helper::ByteArrayVisitor::<
            { elliptic_curve::FieldBytesSize::<NistP256>::USIZE },
        >::new())?;
        match ecdsa::SigningKey::from_bytes(&bytes.into()) {
            Ok(val) => Ok(Self(val)),
            Err(err) => Err(serde::de::Error::custom(err)),
        }
    }
}

impl KeyPair for SigningKey<ecdsa::SigningKey<Secp256k1>> {
    type PublicKey = VerifyingKey<ecdsa::VerifyingKey<Secp256k1>>;
    type Signature = Signature<ecdsa::Signature<Secp256k1>>;
    type Error = signature::Error;

    fn public_key(&self) -> Self::PublicKey {
        VerifyingKey(self.verifying_key().clone())
    }
    fn try_sign(&self, msg: &[u8]) -> Result<Self::Signature, Self::Error> {
        let mut d = Blake2b256::new();
        d.update(msg);
        Ok(Signature(self.0.try_sign_digest(d)?))
    }
}

impl KeyPair for SigningKey<ecdsa::SigningKey<NistP256>> {
    type PublicKey = VerifyingKey<ecdsa::VerifyingKey<NistP256>>;
    type Signature = Signature<ecdsa::Signature<NistP256>>;
    type Error = signature::Error;

    fn public_key(&self) -> Self::PublicKey {
        VerifyingKey(self.verifying_key().clone())
    }
    fn try_sign(&self, msg: &[u8]) -> Result<Self::Signature, Self::Error> {
        let mut d = Blake2b256::new();
        d.update(msg);
        Ok(Signature(self.0.try_sign_digest(d)?))
    }
}

#[derive(Debug, Clone)]
pub struct VerifyingKey<C>(pub(crate) C);

impl<C> core::ops::Deref for VerifyingKey<C> {
    type Target = C;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Serialize for VerifyingKey<ecdsa::VerifyingKey<Secp256k1>> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(&self.0.to_sec1_bytes())
    }
}

impl Serialize for VerifyingKey<ecdsa::VerifyingKey<NistP256>> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(&self.0.to_sec1_bytes())
    }
}

impl<'de> Deserialize<'de> for VerifyingKey<ecdsa::VerifyingKey<Secp256k1>> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes = deserializer.deserialize_bytes(helper::ByteArrayVisitor::<
            { elliptic_curve::sec1::CompressedPointSize::<Secp256k1>::USIZE },
        >::new())?;
        match ecdsa::VerifyingKey::from_sec1_bytes(&bytes) {
            Ok(val) => Ok(Self(val)),
            Err(err) => Err(serde::de::Error::custom(err)),
        }
    }
}

impl<'de> Deserialize<'de> for VerifyingKey<ecdsa::VerifyingKey<NistP256>> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes = deserializer.deserialize_bytes(helper::ByteArrayVisitor::<
            { elliptic_curve::sec1::CompressedPointSize::<NistP256>::USIZE },
        >::new())?;
        match ecdsa::VerifyingKey::from_sec1_bytes(&bytes) {
            Ok(val) => Ok(Self(val)),
            Err(err) => Err(serde::de::Error::custom(err)),
        }
    }
}

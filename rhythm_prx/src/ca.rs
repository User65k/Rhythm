use rcgen::{
    KeyPair, CertificateParams, Certificate, RcgenError,
    DistinguishedName, DnType, SanType, PKCS_ECDSA_P256_SHA256,
    BasicConstraints, ExtendedKeyUsagePurpose, IsCa};
use chrono::{Utc, Duration};
use native_tls::Identity;
use p12::PFX;
use std::net::IpAddr;
use std::collections::HashMap;

use std::fs::{File, OpenOptions};
use std::io::{Read, Write, ErrorKind};
use std::error::Error;
use std::convert::TryFrom;
use std::sync::Arc;
use tokio::sync::Mutex;

const COUNTRY: &str = "DE";
const ORG: &str = "Rhythm";
const CA_CERT: &str = "./ca_cert.der";
const CA_KEY: &str = "./ca_key.der";

#[derive(Clone)]
pub struct CA {
    ca: Arc<Certificate>,
    hosts: Arc<Mutex<HashMap<String, Identity>>>
}

impl CA{
    pub fn new() -> Result<CA, Box<dyn Error>>
    {
        //load or generate
        match File::open(CA_CERT) {
            Ok(mut ca_file) => {
                let mut ca_cert = vec![];
                ca_file.read_to_end(&mut ca_cert)?;

                let mut key_file = File::open(CA_KEY)?;
                let mut ca_key = vec![];
                key_file.read_to_end(&mut ca_key)?;

                let ca_key : &[u8] = &ca_key;
                let ca_cert : &[u8] = &ca_cert;
                let key = KeyPair::try_from(ca_key)?;
                let params = CertificateParams::from_ca_cert_der(ca_cert, key)?;
                Ok(CA {
                    ca: Arc::new(Certificate::from_params(params)?),
                    hosts: Arc::new(Mutex::new(HashMap::new()))
                })
            },
            Err(e) => {
                if e.kind() == ErrorKind::NotFound {
                    let cert = CA::make_ca_cert()?;

                    let mut ca_file = OpenOptions::new().write(true).create_new(true).open(CA_CERT)?;
                    let ca_bytes = cert.serialize_der()?;
                    ca_file.write_all(&ca_bytes)?;

                    let mut key_file = OpenOptions::new().write(true).create(true).truncate(true).open(CA_KEY)?;
                    let privkey_bytes = cert.serialize_private_key_der();
                    key_file.write_all(&privkey_bytes)?;

                    Ok(CA {
                        ca: Arc::new(cert),
                        hosts: Arc::new(Mutex::new(HashMap::new()))
                    })
                }else{
                    Err(Box::new(e))
                }
            }
        }
    }

    pub async fn get_cert_for(&mut self, host_name: &str) -> Result<Identity, RcgenError>
    {
        if let Some(ident) = self.hosts.lock().await.get(host_name) {
            return Ok(ident.clone());
        }

        let mut params = CA::get_params(host_name);
        params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
        params.is_ca = IsCa::SelfSignedOnly;

        let page = Certificate::from_params(params)?;

        let cert_der = page.serialize_der_with_signer(&self.ca)?;
        let key_der = page.serialize_private_key_der();

        let password = "";
        let ca_der = self.ca.serialize_der()?;

        let p12 = PFX::new(&cert_der, &key_der, Some(&ca_der), password, host_name)
            .ok_or_else(|| RcgenError::KeyGenerationUnavailable)?
            .to_der();

        let i = Identity::from_pkcs12(&p12, password).map_err(|_e| RcgenError::KeyGenerationUnavailable)?;

        self.hosts.lock().await.insert(host_name.to_string(),i.clone());
        Ok(i)
    }
    fn get_params(host_name: &str) -> CertificateParams {
        let mut subject_alt_names = vec![];
        if let Ok(addr) = host_name.parse::<IpAddr>() {
            subject_alt_names.push(SanType::IpAddress(addr));
        }else{
            subject_alt_names.push(SanType::DnsName(host_name.to_owned()));
        }
        let mut distinguished_name = DistinguishedName::new();
        distinguished_name.push(DnType::CountryName, COUNTRY);
        distinguished_name.push(DnType::OrganizationName, ORG);
        distinguished_name.push(DnType::CommonName, host_name);
        let mut params = CertificateParams::default();
        params.subject_alt_names = subject_alt_names;
        params.distinguished_name = distinguished_name;
        //params.alg = &PKCS_ECDSA_P256_SHA256;
        params.not_before = Utc::today().and_hms(0,0,0);
        params.not_after = params.not_before + Duration::weeks(1);
        params.serial_number = Some(Utc::now().timestamp_millis() as u64);
        //params.use_authority_key_identifier_extension = true;
        //params.key_identifier_method = KeyIdMethod::Sha512;
        println!("New Cert\tHost: {},\tSN: {}", host_name, params.serial_number.unwrap());
        params
    }

    fn make_ca_cert() -> Result<Certificate, RcgenError> {
        let mut params = CA::get_params(ORG);
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params.extended_key_usages = vec![ExtendedKeyUsagePurpose::Any];
        let ca = Certificate::from_params(params)?;
        Ok(ca)
    }
}

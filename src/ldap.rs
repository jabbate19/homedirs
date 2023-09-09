use ldap3::{drive, Ldap, LdapConnAsync, Scope, SearchEntry};
use rand::prelude::SliceRandom;
use rand::SeedableRng;
use trust_dns_resolver::{
    config::{ResolverConfig, ResolverOpts},
    AsyncResolver,
};

const SEARCH_ATTRS: [&str; 1] = ["homeDirectory"];

#[derive(Clone)]
pub struct LdapClient {
    ldap: Ldap,
}

impl LdapClient {
    pub async fn new(bind_dn: &str, bind_pw: &str) -> Self {
        let servers = get_ldap_servers().await;
        let (conn, mut ldap) = LdapConnAsync::new(
            servers
                .choose(&mut rand::rngs::StdRng::from_entropy())
                .unwrap(),
        )
        .await
        .unwrap();
        drive!(conn);

        ldap.simple_bind(bind_dn, bind_pw).await.unwrap();

        LdapClient { ldap }
    }

    pub async fn get_homedir(&mut self, uid: &str) -> Option<String> {
        self.ldap.with_timeout(std::time::Duration::from_secs(5));
        let (results, _result) = self
            .ldap
            .search(
                "cn=users,cn=accounts,dc=csh,dc=rit,dc=edu",
                Scope::Subtree,
                &format!("uid={uid}"),
                SEARCH_ATTRS,
            )
            .await
            .unwrap()
            .success()
            .unwrap();

        if results.len() == 1 {
            let homedir = SearchEntry::construct(results.get(0).unwrap().to_owned())
                .attrs
                .get("homeDirectory")
                .unwrap()[0]
                .clone();
            Some(homedir)
        } else {
            None
        }
    }
}

async fn get_ldap_servers() -> Vec<String> {
    let resolver = AsyncResolver::tokio(ResolverConfig::default(), ResolverOpts::default());
    let response = resolver.srv_lookup("_ldap._tcp.csh.rit.edu").await.unwrap();

    // TODO: Make sure servers are working
    response
        .iter()
        .map(|record| {
            format!(
                "ldaps://{}",
                record.target().to_string().trim_end_matches('.')
            )
        })
        .collect()
}

use crate::schema::*;

use log::warn;
use pbkdf2::{
    Pbkdf2,
    password_hash::{Encoding, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};
use std::hash::{Hash, Hasher};
use uuid::Uuid;

use diesel::deserialize::{self, FromSql};
use diesel::serialize::{self, Output, ToSql};
use diesel::{
    AsChangeset, AsExpression, ExpressionMethods, FromSqlRow, Identifiable, Insertable,
    PgConnection, QueryDsl, Queryable, RunQueryDsl, pg::Pg,
};
use securefmt::Debug;
use std::collections::HashMap;
use utoipa::ToSchema;

/// Enum representing the role a user has inside our systems. Values are pretty self-explanatory
#[derive(
    Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Debug, AsExpression, FromSqlRow, ToSchema,
)]
#[diesel(sql_type = diesel::sql_types::Integer)]
#[allow(missing_docs)]
pub enum Role {
    EditOrganizationStations = 0,
    CreateOrganizationStations = 1,
    DeleteOrganizationStations = 2,
    EditMaintainedStations = 3,
    CreateMaintainedStations = 4,
    DeleteMaintainedStations = 5,
    EditOrgUserRoles = 6,
    EditOwnOrganization = 7,
    ApproveStations = 8,
}

impl TryFrom<i32> for Role {
    type Error = &'static str;
    fn try_from(role: i32) -> Result<Self, Self::Error> {
        match role {
            0 => Ok(Role::EditOrganizationStations),
            1 => Ok(Role::CreateOrganizationStations),
            2 => Ok(Role::DeleteOrganizationStations),
            3 => Ok(Role::EditMaintainedStations),
            4 => Ok(Role::CreateMaintainedStations),
            5 => Ok(Role::DeleteMaintainedStations),
            6 => Ok(Role::EditOrgUserRoles),
            7 => Ok(Role::EditOwnOrganization),
            8 => Ok(Role::ApproveStations),
            _ => Err("No role corresponding to {role} value!"),
        }
    }
}

impl From<Role> for i32 {
    fn from(val: Role) -> Self {
        match val {
            Role::EditOrganizationStations => 0,
            Role::CreateOrganizationStations => 1,
            Role::DeleteOrganizationStations => 2,
            Role::EditMaintainedStations => 3,
            Role::CreateMaintainedStations => 4,
            Role::DeleteMaintainedStations => 5,
            Role::EditOrgUserRoles => 6,
            Role::EditOwnOrganization => 7,
            Role::ApproveStations => 8,
        }
    }
}

impl Hash for Role {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (*self as i32).hash(state);
    }
}

impl FromSql<diesel::sql_types::Integer, Pg> for Role {
    fn from_sql(bytes: diesel::backend::RawValue<'_, Pg>) -> deserialize::Result<Self> {
        let v: i32 = i32::from_sql(bytes)?;
        let res: Self = v.try_into()?;
        Ok(res)
    }
}

impl ToSql<diesel::sql_types::Integer, Pg> for Role {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        match self {
            Role::EditOrganizationStations => {
                <i32 as ToSql<diesel::sql_types::Integer, Pg>>::to_sql(&0_i32, out)
            }
            Role::CreateOrganizationStations => {
                <i32 as ToSql<diesel::sql_types::Integer, Pg>>::to_sql(&1_i32, out)
            }
            Role::DeleteOrganizationStations => {
                <i32 as ToSql<diesel::sql_types::Integer, Pg>>::to_sql(&2_i32, out)
            }
            Role::EditMaintainedStations => {
                <i32 as ToSql<diesel::sql_types::Integer, Pg>>::to_sql(&3_i32, out)
            }
            Role::CreateMaintainedStations => {
                <i32 as ToSql<diesel::sql_types::Integer, Pg>>::to_sql(&4_i32, out)
            }
            Role::DeleteMaintainedStations => {
                <i32 as ToSql<diesel::sql_types::Integer, Pg>>::to_sql(&5_i32, out)
            }
            Role::EditOrgUserRoles => {
                <i32 as ToSql<diesel::sql_types::Integer, Pg>>::to_sql(&6_i32, out)
            }
            Role::EditOwnOrganization => {
                <i32 as ToSql<diesel::sql_types::Integer, Pg>>::to_sql(&7_i32, out)
            }
            Role::ApproveStations => {
                <i32 as ToSql<diesel::sql_types::Integer, Pg>>::to_sql(&8_i32, out)
            }
        }
    }
}

/// Database struct holding user information
#[derive(Debug, Clone, Deserialize, Queryable, Insertable, AsChangeset, Identifiable, ToSchema)]
#[diesel(table_name = users)]
pub struct User {
    /// Unique identifier for a user.
    pub id: Uuid,
    /// Name of the user.
    pub name: Option<String>,
    /// Email of the user.
    pub email: Option<String>,
    /// Password of the user.
    #[sensitive]
    pub password: String,
    /// This value is interesting for newsletters and other notifications that are distributed via
    /// mail.
    pub email_setting: Option<i32>,
    /// If the user struct is deleted is kept for database consistency.
    pub deactivated: bool,
    /// If user is tlms-wide administrator
    pub admin: bool,
}

/// Database struct holding the relations between organizations and users. Keeps track of user
/// roles within organization
#[derive(Debug, Clone, Deserialize, Queryable, Insertable, AsChangeset, Identifiable)]
#[diesel(table_name = org_users_relations)]
pub struct OrgUsersRelation {
    /// Primary key
    pub id: Uuid,
    /// For which org the role is set
    pub organization: Uuid,
    /// For which user within org the role is set
    pub user_id: Uuid,
    /// The role itself, see [`Roles`] enum for possible values
    pub role: Role,
}

/// Struct used for authenticating users
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthorizedUser {
    /// User Struct
    pub user: User,
    /// Roles that the user has depending on the organization
    pub roles: HashMap<Uuid, Vec<Role>>,
}

/// The UUID of special "community" organization, which is used for crowdsourced stations
pub const COMMUNITY_ORG_ID: Uuid = Uuid::from_u128(0x53e643d7_c300_4de7_ab48_540d08a0cbc6);

/// Database struct holding the information about organizations
#[derive(Debug, Clone, Deserialize, Serialize, Queryable, Insertable, ToSchema)]
#[diesel(table_name = organizations)]
pub struct Organization {
    /// Primary Key
    pub id: Uuid,
    /// Organization Name
    pub name: String,
    /// If Organization information is public
    pub public: bool,
    /// Owner of the organization
    pub owner: Uuid,
    /// Flag that tell if this orga is deleted or not
    pub deactivated: bool,
}

impl AuthorizedUser {
    /// takes a cookie and returnes the corresponging user struct
    pub fn from_postgres(user_id: &Uuid, database_connection: &mut PgConnection) -> Option<Self> {
        use crate::management::org_users_relations::dsl::org_users_relations;
        use crate::management::users::dsl::users;
        use crate::schema::users::id;

        // user struct from currently authenticated user
        // TODO maybe smart doing some little join here
        match users
            .filter(id.eq(user_id))
            .first::<User>(database_connection)
        {
            Ok(found_user) => {
                let associations = org_users_relations
                    .filter(crate::schema::org_users_relations::user_id.eq(user_id))
                    .load::<OrgUsersRelation>(database_connection)
                    .unwrap_or(Vec::new());

                let mut roles: HashMap<Uuid, Vec<Role>> = HashMap::new();

                for association in associations {
                    roles
                        .entry(association.organization)
                        .or_insert_with(Vec::new)
                        .push(association.role);
                }

                Some(AuthorizedUser {
                    user: found_user,
                    roles,
                })
            }
            Err(_) => None,
        }
    }

    /// returns the roles the users has in this organization
    pub fn get_roles(&self, organization: &Uuid) -> Vec<Role> {
        // TODO: optimize useless copy
        self.roles
            .clone()
            .get(organization)
            .unwrap_or(&Vec::new())
            .to_vec()
    }

    /// given a organization and a role returns true if the user has this role
    pub fn allowed(&self, organization: &Uuid, role: &Role) -> bool {
        self.user.admin || self.has_role(organization, role)
    }

    /// returns true if the user has the requested role in the organization
    pub fn has_role(&self, organization: &Uuid, role: &Role) -> bool {
        self.get_roles(organization).contains(role)
    }

    /// returns if the given user is an administrator or not
    pub fn is_admin(&self) -> bool {
        self.user.admin
    }
}

/// custom serializer so we dont accidentailly leak password to the outside
impl Serialize for User {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_struct("User", 5)?;
        s.serialize_field("id", &self.id.to_string())?;
        s.serialize_field("name", &self.name)?;
        s.serialize_field("email", &self.email)?;
        s.serialize_field("email_setting", &self.email_setting)?;
        s.serialize_field("deactivated", &self.deactivated)?;
        s.end()
    }
}

/// Function that takes the plain text passwords and returns the corresponding pbkdf2 hash.
pub fn hash_password(password: &String) -> Option<String> {
    let default_salt_path = String::from("/run/secrets/clicky_bunty_salt");
    let salt_path = std::env::var("SALT_PATH").unwrap_or(default_salt_path);
    let salt = SaltString::b64_encode(std::fs::read(salt_path).unwrap().as_slice()).unwrap();

    match Pbkdf2.hash_password(password.as_bytes(), &salt) {
        Ok(password_hash) => PasswordHash::new(&password_hash.to_string())
            .map(|x| x.to_string())
            .ok(),
        Err(e) => {
            warn!("Unable to hash password: {} with error {:?}", password, e);
            None
        }
    }
}

/// Function that takes plain text passwords and the pbkdf2 hash from the database and returns true
/// if the they correspond to the same password.
pub fn verify_password(password: &String, hashed_password: &str) -> bool {
    let password_hash = match PasswordHash::parse(hashed_password, Encoding::B64) {
        Ok(data) => data,
        Err(e) => {
            warn!("cannot hash password with error {:?}", e);
            return false;
        }
    };
    Pbkdf2
        .verify_password(password.as_bytes(), &password_hash)
        .is_ok()
}

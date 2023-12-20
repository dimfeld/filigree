use uuid::Uuid;

use crate::{
    make_object_id,
    object_id::{ObjectId, ObjectIdPrefix},
};

make_object_id!(UserId, usr);
make_object_id!(OrganizationId, org);
make_object_id!(RoleId, rol);

pub struct UserIdPrefix;
impl ObjectIdPrefix for UserIdPrefix {
    fn prefix() -> &'static str {
        "ui"
    }
}

pub struct AuthUser {
    pub id: UserId,
    pub active: bool,
    pub verified: bool,
}

pub struct AuthRole {
    pub id: RoleId,
}

pub struct AuthOrganization {
    pub id: OrganizationId,
    pub active: bool,
}

pub struct AuthInfo {
    pub user: AuthUser,
    pub organization: AuthOrganization,
    pub roles: Vec<AuthRole>,
    pub all_permissions: Vec<String>,
}

impl AuthInfo {
    pub fn actor_ids(&self) -> Vec<&Uuid> {
        self.roles
            .iter()
            .map(|role| role.id.as_uuid())
            .chain(std::iter::once(self.user.id.as_uuid()))
            .collect()
    }
}

use activitypub::{Actor, activity::{Accept, Follow as FollowAct, Undo}, actor::Person};
use diesel::{self, PgConnection, ExpressionMethods, QueryDsl, RunQueryDsl};

use plume_common::activity_pub::{broadcast, Id, IntoId, inbox::{FromActivity, Notify, WithInbox, Deletable}, sign::Signer};
use blogs::Blog;
use notifications::*;
use users::User;
use schema::follows;

#[derive(Queryable, Identifiable, Associations)]
#[belongs_to(User, foreign_key = "following_id")]
pub struct Follow {
    pub id: i32,
    pub follower_id: i32,
    pub following_id: i32,
    pub ap_url: String,
}

#[derive(Insertable)]
#[table_name = "follows"]
pub struct NewFollow {
    pub follower_id: i32,
    pub following_id: i32,
    pub ap_url: String,
}

impl Follow {
    insert!(follows, NewFollow);
    get!(follows);
    find_by!(follows, find_by_ap_url, ap_url as String);

    pub fn find(conn: &PgConnection, from: i32, to: i32) -> Option<Follow> {
        follows::table.filter(follows::follower_id.eq(from))
            .filter(follows::following_id.eq(to))
            .get_result(conn)
            .ok()
    }

    pub fn into_activity(&self, conn: &PgConnection) -> FollowAct {
        let user = User::get(conn, self.follower_id).unwrap();
        let target = User::get(conn, self.following_id).unwrap();

        let mut act = FollowAct::default();
        act.follow_props.set_actor_link::<Id>(user.clone().into_id()).expect("Follow::into_activity: actor error");
        act.follow_props.set_object_object(user.into_activity(&*conn)).unwrap();
        act.object_props.set_id_string(self.ap_url.clone()).unwrap();
        act.object_props.set_to_link(target.clone().into_id()).expect("New Follow error while setting 'to'");
        act.object_props.set_cc_link_vec::<Id>(vec![]).expect("New Follow error while setting 'cc'");
        act
    }

    /// from -> The one sending the follow request
    /// target -> The target of the request, responding with Accept
    pub fn accept_follow<A: Signer + IntoId + Clone, B: Clone + WithInbox + Actor + IntoId>(
        conn: &PgConnection,
        from: &B,
        target: &A,
        follow: FollowAct,
        from_id: i32,
        target_id: i32
    ) -> Follow {
        let from_url: String = from.clone().into_id().into();
        let target_url: String = target.clone().into_id().into();
        let res = Follow::insert(conn, NewFollow {
            follower_id: from_id,
            following_id: target_id,
            ap_url: format!("{}/follow/{}", from_url, target_url),
        });

        let mut accept = Accept::default();
        let accept_id = format!("{}#accept", follow.object_props.id_string().unwrap_or(String::new()));
        accept.object_props.set_id_string(accept_id).expect("accept_follow: id error");
        accept.object_props.set_to_link(from.clone().into_id()).expect("accept_follow: to error");
        accept.object_props.set_cc_link_vec::<Id>(vec![]).expect("accept_follow: cc error");
        accept.accept_props.set_actor_link::<Id>(target.clone().into_id()).unwrap();
        accept.accept_props.set_object_object(follow).unwrap();
        broadcast(&*target, accept, vec![from.clone()]);
        res
    }
}

impl FromActivity<FollowAct, PgConnection> for Follow {
    fn from_activity(conn: &PgConnection, follow: FollowAct, _actor: Id) -> Follow {
        let from_id = follow.follow_props.actor_link::<Id>().map(|l| l.into())
            .unwrap_or_else(|_| follow.follow_props.actor_object::<Person>().expect("No actor object (nor ID) on Follow").object_props.id_string().expect("No ID on actor on Follow"));
        let from = User::from_url(conn, from_id).unwrap();
        match User::from_url(conn, follow.follow_props.object.as_str().unwrap().to_string()) {
            Some(user) => Follow::accept_follow(conn, &from, &user, follow, from.id, user.id),
            None => {
                let blog = Blog::from_url(conn, follow.follow_props.object.as_str().unwrap().to_string()).unwrap();
                Follow::accept_follow(conn, &from, &blog, follow, from.id, blog.id)
            }
        }
    }
}

impl Notify<PgConnection> for Follow {
    fn notify(&self, conn: &PgConnection) {
        Notification::insert(conn, NewNotification {
            kind: notification_kind::FOLLOW.to_string(),
            object_id: self.id,
            user_id: self.following_id
        });
    }
}

impl Deletable<PgConnection, Undo> for Follow {
    fn delete(&self, conn: &PgConnection) -> Undo {
        diesel::delete(self).execute(conn).expect("Coudn't delete follow");

        let mut undo = Undo::default();
        undo.undo_props.set_actor_link(User::get(conn, self.follower_id).unwrap().into_id()).expect("Follow::delete: actor error");
        undo.object_props.set_id_string(format!("{}/undo", self.ap_url)).expect("Follow::delete: id error");
        undo.undo_props.set_object_object(self.into_activity(conn)).expect("Follow::delete: object error");
        undo
    }

    fn delete_id(id: String, conn: &PgConnection) {
        if let Some(follow) = Follow::find_by_ap_url(conn, id) {
            follow.delete(conn);
        }
    }
}

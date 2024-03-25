mod mocks;

mod misc {

    use crate::routes::health;

    use actix_web::body::MessageBody;
    use actix_web::test::TestRequest;
    use actix_web::{test, App};

    #[actix_web::test]
    async fn health_endpoint() {
        let app = test::init_service(App::new().service(health)).await;
        let req = TestRequest::get().uri("/health").to_request();
        let (_req, res) = test::call_service(&app, req).await.into_parts();
        let status = res.status();
        let body = res.into_body().try_into_bytes().unwrap();
        assert!(status.is_success());
        assert_eq!(body, "👍️\n");
    }
}

mod auth;

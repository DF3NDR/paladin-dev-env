# API Coverage — RustFS S3 API (through `rust-s3` 0.35.1 in `MinioAdapter`)

> Full coverage by default. Opt-outs are explicit, reasoned decisions.
> Scope: the S3 capability surface `crates/paladin-storage/src/minio.rs` touches when it runs
> against `rustfs/rustfs:1.0.0` (dev/test/CI and the `k8s/rustfs.yaml` reference manifest).
> The adapter stays a generic SigV4 S3 client (Phase 45 D-08, D-10); every row below is also what
> it sends to MinIO, AWS S3 or DigitalOcean Spaces in production.

| capability | decision | reason |
|---|---|---|
| bucket existence check (ListObjectsV2 on the configured bucket) | INTEGRATE | `ensure_bucket_exists` runs on every `MinioAdapter::new` (D-05) |
| bucket create (path-style `PUT /{bucket}` with `BucketConfiguration::default()`) | INTEGRATE | replaces the `mc` bootstrap; switched to `Bucket::create_with_path_style` in plan 45-01 (D-05) |
| put object (`upload_file`, content type from the path) | INTEGRATE | contract suite `upload_download`, `file_operations`, `batch_operations` |
| get object (`download_file`) | INTEGRATE | contract suite `upload_download` and the multipart download check |
| head object (`get_file_info`, `file_exists`) | INTEGRATE | contract suite; the ETag assertion reads it (D-09 b) |
| list objects (`list_files`, storage statistics) | INTEGRATE | contract suite `storage_stats`; the ETag assertion compares it with head (D-09 b) |
| delete object (`delete_file`, batch delete, suite cleanup) | INTEGRATE | contract suite `file_operations`, `batch_operations`, `cleanup` |
| copy object (`copy_file`, `move_file`) | INTEGRATE | bucket double-prefix fixed in plan 45-01; contract suite `file_operations` |
| presigned PUT URL (`generate_upload_url`) | INTEGRATE | exercised with a real HTTP `PUT` through the URL (D-09 c) |
| presigned GET URL (`generate_download_url`) | INTEGRATE | exercised with a real HTTP `GET` through the URL (D-09 c) |
| multipart create (`create_multipart_upload`) | INTEGRATE | returns a stateless upload token (plan 45-01, D-09 a) |
| multipart upload part (`upload_part`) | INTEGRATE | implemented with `put_multipart_chunk` (D-09 a) |
| multipart complete (`complete_multipart_upload`) | INTEGRATE | implemented with `complete_multipart_upload`; a 200-with-`<Error>` body is treated as a failure (D-09 a) |
| multipart abort (`abort_multipart_upload`) | INTEGRATE | implemented with `abort_upload`; the abort case asserts the object is absent (D-09 a) |
| ETag as an opaque, quote-stripped, stable token (`FileItem.md5_hash` label) | INTEGRATE | asserted equal across head and list and changed by an overwrite, never as a content MD5 (D-09 b) |
| health check (`health_check` list probe) | INTEGRATE | contract suite `health_check` |
| server health endpoints (`GET /health`, `GET /health/ready`) | INTEGRATE | used by every compose healthcheck, CI `--health-cmd`, k8s probes and the local-mode readiness poll (D-06, D-07) |
| S3 object versioning API (`PutBucketVersioning`, `versionId` reads) | OPT-OUT | explicitly out of scope: `FileVersioningPort` on this adapter emulates versions with timestamped keys and no phase requirement names S3 versioning |
| bucket policy (anonymous/public read) | OPT-OUT | deliberately dropped with the `mc` bootstrap: nothing reads `paladin-files` anonymously, and removing the public policy is security-positive (RESEARCH Pitfall 13) |
| object tagging API (`PutObjectTagging`) | OPT-OUT | not needed: the adapter carries `UploadOptions.tags` as object metadata, not S3 tags, and no requirement asks for tag queries |
| server-side encryption (SSE-S3 / SSE-KMS / SSE-C headers) | OPT-OUT | explicitly out of scope: the adapter sends no SSE headers today and no phase requirement adds encryption at rest |
| object lock / retention / legal hold | OPT-OUT | not needed: no retention requirement exists for dev/test/CI or the reference store |
| lifecycle rules, bucket notifications, bucket CORS configuration | OPT-OUT | not needed: the adapter never configures a bucket beyond creating it |
| list in-progress multipart uploads / list parts | OPT-OUT | not needed yet: the port has no listing verb for uploads; an abandoned upload is aborted by its caller through the token |
| virtual-hosted-style addressing (`{bucket}.{host}`) | OPT-OUT | explicitly out of scope: RustFS answers `501 NotImplemented` without `RUSTFS_SERVER_DOMAINS`; the adapter is path-style end to end (`MinioConfig.path_style: true`) |
| RustFS admin API, IAM users/policies, STS | OPT-OUT | not needed: the adapter authenticates with the store's root credentials in dev/test/CI; per-application IAM users are an operator concern documented in ADR-0055 |
| RustFS web console (`:9001/rustfs/console/`) | OPT-OUT | not an adapter capability: enabled only in the dev compose and devcontainer for operator convenience, disabled (`RUSTFS_CONSOLE_ENABLE=false`) in CI, the test compose and the k8s reference manifest |

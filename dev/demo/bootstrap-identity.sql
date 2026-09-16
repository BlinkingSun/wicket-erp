-- bootstrap-identity.sql — dev-only demo login (idempotent).
-- Applied by `just demo` after `wicket migrate`, not by `db-reset`.
-- Must run as one transaction so wicket.txid matches pg_current_xact_id().

\set ON_ERROR_STOP on

BEGIN;

SELECT pg_catalog.set_config('demo.login_hash', :'login_hash', true);

SELECT
  pg_catalog.set_config('wicket.actor_id',      '00000000-0000-4000-8000-000000000001', true),
  pg_catalog.set_config('wicket.actor_kind',    'service', true),
  pg_catalog.set_config('wicket.actor_display', 'system', true),
  pg_catalog.set_config('wicket.txid',          pg_catalog.pg_current_xact_id()::text, true),
  pg_catalog.set_config('wicket.action',        'demo.bootstrap', true),
  pg_catalog.set_config('wicket.source_kind',   'maintenance', true);

INSERT INTO identity.role_permission (role_id, permission_key)
SELECT r.id, p.key
  FROM identity.role r
 CROSS JOIN (VALUES
   ('items.edit'),
   ('items.release'),
   ('locations.view'),
   ('locations.edit'),
   ('lots.edit'),
   ('lots.release'),
   ('inventory.receive'),
   ('inventory.issue'),
   ('inventory.move'),
   ('inventory.count'),
   ('inventory.adjust'),
   ('production.create'),
   ('production.release'),
   ('production.issue'),
   ('production.complete')
 ) AS p(key)
 WHERE r.name = 'admin'
ON CONFLICT DO NOTHING;

DO $$
DECLARE
  demo_id uuid := '00000000-0000-4000-8000-0000000000d1';
  admin_role uuid;
  login_hash text := current_setting('demo.login_hash', true);
BEGIN
  IF login_hash IS NULL OR login_hash = '' THEN
    RAISE EXCEPTION 'demo bootstrap: login_hash psql variable is required';
  END IF;

  SELECT id INTO demo_id
    FROM identity.principal
   WHERE lower(username) = lower('demo');
  IF demo_id IS NULL THEN
    demo_id := '00000000-0000-4000-8000-0000000000d1';
    INSERT INTO identity.principal
        (id, kind, username, display_name, status, created_at)
    VALUES
        (demo_id, 'user', 'demo', 'Demo Operator', 'active', now());
  END IF;

  INSERT INTO identity.login_credential
      (principal_id, hash, m, t, p, rotated_at, failed_attempts)
  VALUES
      (demo_id, login_hash, 19456, 2, 1, now(), 0)
  ON CONFLICT (principal_id) DO NOTHING;

  SELECT id INTO admin_role FROM identity.role WHERE name = 'admin';
  IF admin_role IS NULL THEN
    RAISE EXCEPTION 'demo bootstrap: admin role missing (run wicket migrate first)';
  END IF;

  INSERT INTO identity.principal_role (principal_id, role_id)
  VALUES (demo_id, admin_role)
  ON CONFLICT DO NOTHING;
END $$;

COMMIT;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'carteira_owner') THEN
        CREATE ROLE carteira_owner
            LOGIN
            SUPERUSER
            PASSWORD 'carteira_owner_dev_password';
    ELSE
        ALTER ROLE carteira_owner
            LOGIN
            SUPERUSER
            PASSWORD 'carteira_owner_dev_password';
    END IF;
END
$$;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'carteira_migrator') THEN
        CREATE ROLE carteira_migrator LOGIN PASSWORD 'carteira_migrator_dev_password';
    ELSE
        ALTER ROLE carteira_migrator LOGIN PASSWORD 'carteira_migrator_dev_password';
    END IF;
END
$$;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'carteira_runtime') THEN
        CREATE ROLE carteira_runtime LOGIN PASSWORD 'carteira_runtime_dev_password';
    ELSE
        ALTER ROLE carteira_runtime LOGIN PASSWORD 'carteira_runtime_dev_password';
    END IF;
END
$$;

GRANT CONNECT ON DATABASE carteira_dev TO carteira_migrator;
GRANT CONNECT ON DATABASE carteira_dev TO carteira_runtime;

ALTER SCHEMA public OWNER TO carteira_migrator;
REVOKE CREATE ON SCHEMA public FROM PUBLIC;
REVOKE CREATE ON SCHEMA public FROM carteira_runtime;

GRANT USAGE, CREATE ON SCHEMA public TO carteira_migrator;
GRANT USAGE ON SCHEMA public TO carteira_runtime;

ALTER DEFAULT PRIVILEGES FOR ROLE carteira_migrator IN SCHEMA public
    GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO carteira_runtime;

ALTER DEFAULT PRIVILEGES FOR ROLE carteira_migrator IN SCHEMA public
    GRANT USAGE, SELECT ON SEQUENCES TO carteira_runtime;

DO $$
DECLARE
    object_name TEXT;
BEGIN
    FOR object_name IN
        SELECT quote_ident(tablename)
        FROM pg_tables
        WHERE schemaname = 'public'
    LOOP
        EXECUTE format('ALTER TABLE public.%s OWNER TO carteira_migrator', object_name);
    END LOOP;

    FOR object_name IN
        SELECT quote_ident(sequencename)
        FROM pg_sequences
        WHERE schemaname = 'public'
    LOOP
        EXECUTE format('ALTER SEQUENCE public.%s OWNER TO carteira_migrator', object_name);
    END LOOP;
END
$$;

GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO carteira_runtime;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO carteira_runtime;

ALTER ROLE carteira_migrator NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION;
ALTER ROLE carteira_runtime NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION;

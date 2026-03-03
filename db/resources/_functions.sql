--
-- TOC entry 329 (class 1255 OID 62767)
-- Name: resource_update_trigger(); Type: FUNCTION; Schema: resources; Owner: -
--

CREATE FUNCTION resource_update_trigger() RETURNS trigger
    LANGUAGE plpgsql
    AS $_$declare
    _restype resources."EResourceType";
    _key_col varchar;
    _version integer;
    _pending boolean;
    _added integer;
    _invalidated integer;
begin
    select "type", key_column
    into strict _restype, _key_col
    from resources.resource_types
    where "table" = TG_TABLE_NAME;

    select "version", pending or (snapshot = txid_current_snapshot()::varchar)
    into strict _version, _pending
    from resources.resource_versions
    where "type" = _restype
    order by (snapshot = txid_current_snapshot()::varchar) desc, "version" desc
    fetch first row only;


    if (TG_OP = 'DELETE' or TG_OP = 'UPDATE') then
        execute 'SELECT $1.' || quote_ident(_key_col)
        using old
        into _invalidated;
    else
        execute 'SELECT $1.' || quote_ident(_key_col)
        using new
        into _added;
    end if;

    if (_pending) then
        update resources.resource_versions
        set invalidated_keys = 
            case 
                when _invalidated is not null then array_append(invalidated_keys, _invalidated)
                else array(select unnest(invalidated_keys) except select _added)
            end,
            new_keys = 
            case
                when _added is not null then array_append(new_keys, _added) 
                else array(select unnest(new_keys) except select _invalidated)
            end
        where version = _version and type = _restype;
    else
        insert into resources.resource_versions (type, version, invalidated_keys, new_keys)
        values (_restype, _version + 1, 
                case when _invalidated is not null then array[_invalidated] else array[]::integer[] end,
                case when _added is not null then array[_added] else array[]::integer[] end);
    end if;

    if (TG_OP = 'DELETE') then
        return old;
    else
        return new;
    end if;
end$_$;


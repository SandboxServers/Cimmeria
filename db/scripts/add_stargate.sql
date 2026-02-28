do $$
declare
   evtset_id integer;
   seq_id integer;
   evt_id integer;
begin
  insert into resources.event_sets (name)
  values ('Agnos')
  returning event_set_id
  into evtset_id;

  for seq_id, evt_id in
    insert into resources.sequences (event_id, kismet_script_name)
    select (6000 + s, 'NAME.Main_Sequence.Prefabs.GLB-Stargate_Prefab_Seq'
    from generate_series(0, 13) s
    returning sequence_id, event_id
  loop
    insert into resources.event_sets_instances (event_set_id, event_id, sequence_id)
    values (evtset_id, evt_id, seq_id);
  end loop;

  update resources.stargates
  set event_set_id = evtset_id
  where name = 'Agnos';
end $$
language 'plpgsql';
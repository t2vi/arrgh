UPDATE title_meta SET cover_local_path = NULL WHERE cover_local_path LIKE './downloads/%';
UPDATE manga SET cover_url = NULL WHERE cover_url LIKE './downloads/%';

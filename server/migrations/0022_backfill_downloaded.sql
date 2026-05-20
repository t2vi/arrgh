UPDATE chapters SET downloaded = 1
WHERE downloaded = 0
  AND EXISTS (
    SELECT 1 FROM download_queue
    WHERE chapter_id = chapters.id AND status = 'done'
  );

UPDATE manga SET content_type = 'novel'  WHERE source IN ('royalroad', 'novelfull') AND content_type = 'manga';
UPDATE manga SET content_type = 'manhwa' WHERE source = 'toonily'                   AND content_type = 'manga';

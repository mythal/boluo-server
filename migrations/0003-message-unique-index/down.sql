ALTER TABLE messages DROP CONSTRAINT IF EXISTS "order_index_unique";
DROP INDEX IF EXISTS order_index;
CREATE INDEX "order_index" ON messages (order_date DESC, order_offset ASC);

DROP INDEX "order_index";
CREATE INDEX "order_index" ON messages (order_date DESC, order_offset DESC);
ALTER TABLE messages ADD CONSTRAINT order_index_unique UNIQUE (channel_id, order_date, order_offset) DEFERRABLE INITIALLY IMMEDIATE;
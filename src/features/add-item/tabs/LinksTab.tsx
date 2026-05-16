import { useState } from "react";
import type { DockItemInput } from "../../dock/types";

type LinksTabProps = {
  onAddItem: (item: DockItemInput) => void;
};

export function LinksTab({ onAddItem }: LinksTabProps) {
  const [url, setUrl] = useState("");
  const [linkLabel, setLinkLabel] = useState("");

  function handleAdd() {
    if (!url.trim()) return;
    onAddItem({
      type: "url",
      label: linkLabel.trim() || url.trim(),
      url: url.trim(),
    });
    setUrl("");
    setLinkLabel("");
  }

  return (
    <div className="add-item-form">
      <label className="add-item-label">
        URL
        <input
          className="add-item-input"
          type="text"
          placeholder="https://example.com"
          value={url}
          onChange={(e) => setUrl(e.target.value)}
        />
      </label>
      <label className="add-item-label">
        Label (optional)
        <input
          className="add-item-input"
          type="text"
          placeholder="My Link"
          value={linkLabel}
          onChange={(e) => setLinkLabel(e.target.value)}
        />
      </label>
      <button className="add-item-submit" onClick={handleAdd} disabled={!url.trim()}>
        Add Link
      </button>
    </div>
  );
}

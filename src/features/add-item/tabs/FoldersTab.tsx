import { useState } from "react";
import type { DockItemInput } from "../../dock/types";

type FoldersTabProps = {
  onAddItem: (item: DockItemInput) => void;
};

export function FoldersTab({ onAddItem }: FoldersTabProps) {
  const [folderPath, setFolderPath] = useState("");
  const [folderLabel, setFolderLabel] = useState("");

  function handleAdd() {
    if (!folderPath.trim()) return;
    onAddItem({
      type: "folder",
      label: folderLabel.trim() || folderPath.split("/").pop() || "Folder",
      folderPath: folderPath.trim(),
    });
    setFolderPath("");
    setFolderLabel("");
  }

  return (
    <div className="add-item-form">
      <label className="add-item-label">
        Folder Path
        <input
          className="add-item-input"
          type="text"
          placeholder="/Users/you/Documents"
          value={folderPath}
          onChange={(e) => setFolderPath(e.target.value)}
        />
      </label>
      <label className="add-item-label">
        Label (optional)
        <input
          className="add-item-input"
          type="text"
          placeholder="My Folder"
          value={folderLabel}
          onChange={(e) => setFolderLabel(e.target.value)}
        />
      </label>
      <button className="add-item-submit" onClick={handleAdd} disabled={!folderPath.trim()}>
        Add Folder
      </button>
    </div>
  );
}

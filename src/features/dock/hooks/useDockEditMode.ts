import { useCallback, useState } from "react";

export function useDockEditMode() {
  const [isEditing, setIsEditing] = useState(false);

  const enterEditMode = useCallback(() => {
    setIsEditing(true);
  }, []);

  const exitEditMode = useCallback(() => {
    setIsEditing(false);
  }, []);

  const toggleEditMode = useCallback(() => {
    setIsEditing((prev) => !prev);
  }, []);

  return {
    isEditing,
    enterEditMode,
    exitEditMode,
    toggleEditMode,
  };
}

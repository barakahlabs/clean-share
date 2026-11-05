import { BottomSheet } from "../components/BottomSheet";
import { BookmarkForm } from "./BookmarkForm";

type Props = {
  text: string;
  /** Called after saving (`true`) or closing (`false`). */
  onDone: (saved: boolean) => void;
};

/** The bookmark form in a full-height sheet, for editing a saved bookmark (its link stays as is). */
export function SaveBookmarkSheet({ text, onDone }: Props) {
  return (
    <BottomSheet labelledBy="bookmark-form-title" tall onClose={() => onDone(false)}>
      <BookmarkForm text={text} editableLink={false} onClose={() => onDone(false)} onSaved={() => onDone(true)} />
    </BottomSheet>
  );
}

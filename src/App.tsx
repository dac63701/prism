import { MemoryRouter, Routes, Route } from "react-router-dom";
import AppLayout from "@/components/layout/AppLayout";
import HomePage from "@/pages/HomePage";
import LibraryPage from "@/pages/LibraryPage";
import ClipDetailPage from "@/pages/ClipDetailPage";
import SettingsPage from "@/pages/SettingsPage";

function App() {
  return (
    <MemoryRouter>
      <Routes>
        <Route element={<AppLayout />}>
          <Route path="/" element={<HomePage />} />
          <Route path="/library" element={<LibraryPage />} />
          <Route path="/clip/:filename" element={<ClipDetailPage />} />
          <Route path="/settings" element={<SettingsPage />} />
        </Route>
      </Routes>
    </MemoryRouter>
  );
}

export default App;

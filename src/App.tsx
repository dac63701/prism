import { MemoryRouter, Routes, Route } from "react-router-dom";
import AppLayout from "@/components/layout/AppLayout";
import HomePage from "@/pages/HomePage";
import LibraryPage from "@/pages/LibraryPage";
import ClipDetailPage from "@/pages/ClipDetailPage";
import SettingsPage from "@/pages/SettingsPage";
import ErrorBoundary from "@/components/common/ErrorBoundary";

function App() {
  return (
    <MemoryRouter>
      <ErrorBoundary>
        <Routes>
          <Route element={<AppLayout />}>
            <Route path="/" element={<HomePage />} />
            <Route path="/library" element={<LibraryPage />} />
            <Route path="/clip/:filename" element={<ClipDetailPage />} />
            <Route path="/settings" element={<SettingsPage />} />
          </Route>
        </Routes>
      </ErrorBoundary>
    </MemoryRouter>
  );
}

export default App;

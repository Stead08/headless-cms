// import jsonServerProvider from 'ra-data-json-server';
import { Route, Routes } from 'react-router-dom';
import Home from './routes/home.tsx';
import Dashboard from './routes/dashboard.tsx';
import Explore from './components/Explore.tsx';
import Favorites from './components/Favorites.tsx';
import Settings from './components/Settings.tsx';
import SimpleSidebar from './components/Sidebar.tsx';
import Trending from './components/Trending.tsx';
import CommonHeader from './components/CommonHeader.tsx';

// const dataProvider = jsonServerProvider('https://jsonplaceholder.typicode.com');

const App = () => {
  return (
    <div className="App">
      <CommonHeader />
      <div
        style={{ display: 'flex', flexDirection: 'column', height: '100vh' }}
      >
        <div style={{ display: 'flex', flex: 1, overflow: 'hidden' }}>
          <SimpleSidebar children="" />
          <div style={{ flex: 1, overflow: 'auto' }}>
            <Routes>
              <Route path="/" element={<Home />} />
              <Route path="/dashboard" element={<Dashboard />} />
              <Route path="/explore" element={<Explore />} />
              <Route path="/favorites" element={<Favorites />} />
              <Route path="/settings" element={<Settings />} />
              <Route path="/trending" element={<Trending />} />
              <Route path="*" element={<h1>Not Found</h1>} />
            </Routes>
          </div>
        </div>
      </div>
    </div>
  );
};

export default App;

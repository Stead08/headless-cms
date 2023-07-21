const MainContent = () => {
  return (
    <div>
      <h1>Welcome to the Dashboard</h1>
      {Array.from({ length: 100 }).map((_, i) => (
        <p key={i}>This is the main content of the dashboard. Line {i + 1}</p>
      ))}
    </div>
  );
};

export default MainContent;

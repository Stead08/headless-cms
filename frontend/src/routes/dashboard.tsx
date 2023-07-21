import { useAuth0 } from '@auth0/auth0-react';
import { Link } from 'react-router-dom';
export const Dashboard = () => {
  const { user, isAuthenticated, isLoading, getAccessTokenSilently } =
    useAuth0();
  const healthCheckHandler = async (e: React.MouseEvent<HTMLButtonElement>) => {
    e.preventDefault();
    try {
      const accessToken = await getAccessTokenSilently({
        authorizationParams: {
          audience: 'http://localhost:8080',
        },
      });
      const response = await fetch('/api/service/health', {
        headers: {
          Authorization: `Bearer ${accessToken}`,
        },
      });
      console.log(response);
    } catch (error) {
      console.error(error);
    }
  };
  if (!user) {
    return (
      <div>
        <h2>Please Login</h2>
        <Link to="/">Home</Link>
      </div>
    );
  }
  if (isLoading) {
    return <div>Loading ...</div>;
  }

  return (
    isAuthenticated && (
      <>
        <h1>Dashboard</h1>
        <button
          onClick={(e) => {
            const event = e;
            // Call the async function without waiting for it
            healthCheckHandler(event).catch((error) =>
              console.error('failed to fetch', error),
            );
          }}
        >
          Health Check
        </button>
      </>
    )
  );
};

export default Dashboard;

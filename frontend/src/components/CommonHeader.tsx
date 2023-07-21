import { useEffect, useRef } from 'react';
import { useAuth0 } from '@auth0/auth0-react';
import Header from '../../../common-ui/dist/Header.svelte';

const CommonHeader = () => {
  const { user, logout } = useAuth0();
  const containerRef = useRef(null);
  useEffect(() => {
    if (containerRef.current) {
      const header = new Header({
        target: containerRef.current,
        props: {
          user,
        },
      });
      const handleLogout = () =>
        logout({ logoutParams: { returnTo: window.location.origin } });
      header.$on('logout', handleLogout);

      return () => {
        header.$on('logout', handleLogout);
        header.$destroy();
      };
    }
  });
  return <div ref={containerRef}></div>;
};

export default CommonHeader;

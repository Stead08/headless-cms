import SignUp from "../islands/SignUp.tsx";
import Header from "../components/Header.tsx";

export const SignUpPage = () => {
    return (
        <>
            <Header active={"SignUp"}/>
            <SignUp />
        </>
    )
}

export default SignUpPage;
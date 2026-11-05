use sigmaos_sandcrawler::*;

#[test]
fn component_is_constructible_and_runnable() {
    // Arrange
    let c = sigmaos_sandcrawler::Sandcrawler::new("sigmaos-sandcrawler").expect("valid name");
    // Act
    let out = c.run().expect("run succeeds");
    // Assert
    assert!(out.contains("sigmaos-sandcrawler"));
}
